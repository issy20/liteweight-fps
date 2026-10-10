use glam::Mat4;
use liteweight_fps::{
    engine::{renderer::SceneRenderer, text::FontAtlas},
    Scene, Vertex,
};
use wgpu::util::DeviceExt;

#[test]
#[ignore = "GPU実機で明示実行: cargo test --test m1_gpu -- --ignored"]
fn nearer_surface_wins_in_both_draw_orders() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(&Default::default());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("GPU adapter required");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await
            .unwrap();
        for reverse in [false, true] {
            let mut vertices = Vec::new();
            for (z, color) in if reverse {
                [(0.8, [0., 0., 1.]), (0.2, [1., 0., 0.])]
            } else {
                [(0.2, [1., 0., 0.]), (0.8, [0., 0., 1.])]
            } {
                for [x, y] in [[-1., -1.], [3., -1.], [-1., 3.]] {
                    vertices.push(Vertex {
                        position: [x, y, z],
                        color: [color[0], color[1], color[2], 1.0],
                    });
                }
            }
            let mesh = Scene {
                vertices,
                indices: vec![0, 1, 2, 3, 4, 5],
            };
            let renderer =
                SceneRenderer::new(&device, &mesh, 32, 32, wgpu::TextureFormat::Rgba8Unorm)
                    .unwrap();
            let mut encoder = device.create_command_encoder(&Default::default());
            renderer.encode_scene(&queue, &mut encoder, Mat4::IDENTITY);
            let readback = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: &[0; 256 * 32],
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            });
            encoder.copy_texture_to_buffer(
                wgpu::TexelCopyTextureInfo {
                    texture: renderer.color_texture(),
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyBufferInfo {
                    buffer: &readback,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(256),
                        rows_per_image: Some(32),
                    },
                },
                wgpu::Extent3d {
                    width: 32,
                    height: 32,
                    depth_or_array_layers: 1,
                },
            );
            queue.submit(Some(encoder.finish()));
            let slice = readback.slice(..);
            let (sender, receiver) = std::sync::mpsc::channel();
            slice.map_async(wgpu::MapMode::Read, move |result| {
                sender.send(result).unwrap()
            });
            device.poll(wgpu::Maintain::Wait);
            receiver.recv().unwrap().unwrap();
            let bytes = slice.get_mapped_range();
            // 中央ピクセルは手前の赤面が勝つ。細い表面模様（最大6%暗く）が掛かっても
            // 赤成分が支配的であることを確認する。
            let px = &bytes[16 * 256 + 16 * 4..16 * 256 + 16 * 4 + 4];
            assert!(
                px[0] >= 230 && px[1] < 40 && px[2] < 40 && px[3] == 255,
                "中央ピクセルが手前の赤面になっていない: {px:?}"
            );
        }
    });
}

#[test]
#[ignore = "GPU実機で明示実行: cargo test --test m1_gpu -- --ignored"]
fn resized_targets_render_with_matching_depth_and_letterboxed_presentation() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(&Default::default());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("GPU adapter required");
        let (device, queue) = adapter
            .request_device(&Default::default(), None)
            .await
            .unwrap();
        let mesh = Scene {
            vertices: [[-1., -1., 0.2], [3., -1., 0.2], [-1., 3., 0.2]]
                .map(|position| Vertex {
                    position,
                    color: [1., 0., 0., 1.],
                })
                .to_vec(),
            indices: vec![0, 1, 2],
        };
        let mut renderer =
            SceneRenderer::new(&device, &mesh, 32, 32, wgpu::TextureFormat::Rgba8Unorm).unwrap();
        // 同一インスタンスでリサイズと無効値拒否を繰り返す。
        for (width, height) in [(32, 16), (16, 32), (32, 16)] {
            renderer.set_internal_size(&device, width, height).unwrap();
            assert!(renderer.set_internal_size(&device, 0, height).is_err());
            assert_eq!(
                renderer.color_texture().size(),
                wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1
                }
            );
            renderer.resize_display(0, 0);
            assert_eq!(renderer.layout().viewport(), None);
            renderer.resize_display(64, 64);
            let output = device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size: wgpu::Extent3d {
                    width: 64,
                    height: 64,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let view = output.create_view(&Default::default());
            let mut encoder = device.create_command_encoder(&Default::default());
            renderer.encode_scene(&queue, &mut encoder, Mat4::IDENTITY);
            renderer.encode_present(&mut encoder, &view);
            let readback = device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: 256 * 64,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            encoder.copy_texture_to_buffer(
                wgpu::TexelCopyTextureInfo {
                    texture: &output,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyBufferInfo {
                    buffer: &readback,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(256),
                        rows_per_image: Some(64),
                    },
                },
                output.size(),
            );
            queue.submit(Some(encoder.finish()));
            let (sender, receiver) = std::sync::mpsc::channel();
            readback
                .slice(..)
                .map_async(wgpu::MapMode::Read, move |r| sender.send(r).unwrap());
            device.poll(wgpu::Maintain::Wait);
            receiver.recv().unwrap().unwrap();
            let bytes = readback.slice(..).get_mapped_range();
            // 中央は赤面。細い表面模様（最大6%暗く）が掛かっても赤成分が支配的。
            let px = &bytes[32 * 256 + 32 * 4..32 * 256 + 32 * 4 + 4];
            assert!(
                px[0] >= 230 && px[1] < 40 && px[2] < 40 && px[3] == 255,
                "中央ピクセルが赤面になっていない: {px:?}"
            );
            // 黒帯部分は黒のまま。
            assert_eq!(&bytes[..4], &[0, 0, 0, 255]);
        }

        renderer
            .resize_to_display_aspect(&device, 64, 64, 1.0)
            .unwrap();
        assert_eq!(renderer.layout().viewport(), Some((0, 0, 64, 64)));
        assert_eq!(renderer.layout().internal_size(), (64, 64));
        assert_eq!(renderer.color_texture().size().width, 64);
        assert_eq!(renderer.color_texture().size().height, 64);
    });
}

#[test]
#[ignore = "GPU実機で明示実行: cargo test --test m1_gpu text_atlas_draws -- --ignored"]
fn text_atlas_draws_into_the_hud_target() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(&Default::default());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("GPU adapter required");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await
            .unwrap();
        let world = Scene {
            vertices: [[-1., -1., 0.2], [3., -1., 0.2], [-1., 3., 0.2]]
                .map(|position| Vertex {
                    position,
                    color: [0.1, 0.1, 0.1, 1.0],
                })
                .to_vec(),
            indices: vec![0, 1, 2],
        };
        let mut renderer =
            SceneRenderer::new(&device, &world, 64, 64, wgpu::TextureFormat::Rgba8Unorm).unwrap();
        let layout = FontAtlas::shared().layout("Readable", [8.0, 8.0], 24.0, [64, 64], [1.0; 4]);
        assert!(!layout.mesh.indices.is_empty());
        renderer.prepare_text(&device, &queue);
        renderer.update_text(&device, &queue, &layout.mesh);
        let mut encoder = device.create_command_encoder(&Default::default());
        renderer.encode_scene(&queue, &mut encoder, Mat4::IDENTITY);
        renderer.encode_extras(&queue, &mut encoder, Mat4::IDENTITY);
        queue.submit(Some(encoder.finish()));
        device.poll(wgpu::Maintain::Wait);
    });
}
