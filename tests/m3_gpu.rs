use glam::Mat4;
use liteweight_fps::{engine::renderer::SceneRenderer, Scene, Vertex};

fn quad(mesh: &mut Scene, left: f32, right: f32, bottom: f32, top: f32, z: f32, color: [f32; 3]) {
    let base = mesh.vertices.len() as u16;
    for (x, y) in [(left, bottom), (right, bottom), (right, top), (left, top)] {
        mesh.vertices.push(Vertex {
            position: [x, y, z],
            color,
        });
    }
    mesh.indices
        .extend([base, base + 1, base + 2, base, base + 2, base + 3]);
}
#[test]
#[ignore = "GPU実機で明示実行: cargo test --test m3_gpu -- --ignored"]
fn targets_respect_world_depth_and_hud_overlays_both() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(&Default::default());
        let adapter = instance
            .request_adapter(&Default::default())
            .await
            .expect("GPU adapter required");
        let (device, queue) = adapter
            .request_device(&Default::default(), None)
            .await
            .unwrap();
        let empty = || Scene {
            vertices: vec![],
            indices: vec![],
        };
        let mut world = empty();
        quad(&mut world, -1., 1., -1., 1., 0.4, [1., 0., 0.]);
        let mut extras = empty();
        quad(&mut extras, -1., 0., -1., 1., 0.2, [0., 1., 0.]);
        quad(&mut extras, 0., 1., -1., 1., 0.6, [0., 1., 0.]);
        let mut hud = empty();
        quad(&mut hud, -0.2, 0.2, -1., 1., 0.9, [0., 0., 1.]);
        let mut renderer =
            SceneRenderer::new(&device, &world, 32, 32, wgpu::TextureFormat::Rgba8Unorm).unwrap();
        renderer.update_extras(&queue, &extras, &hud);
        let mut encoder = device.create_command_encoder(&Default::default());
        renderer.encode_scene(&queue, &mut encoder, Mat4::IDENTITY);
        renderer.encode_extras(&queue, &mut encoder, Mat4::IDENTITY);
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 256 * 32,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
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
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
        device.poll(wgpu::Maintain::Wait);
        rx.recv().unwrap().unwrap();
        let bytes = slice.get_mapped_range();
        for (x, channel) in [(4, 1), (27, 0), (16, 2)] {
            let pixel = &bytes[16 * 256 + x * 4..16 * 256 + x * 4 + 4];
            assert!(pixel[channel] >= 230, "x={x}: {pixel:?}");
            for (c, value) in pixel.iter().take(3).enumerate() {
                if c != channel {
                    assert_eq!(*value, 0);
                }
            }
        }
    });
}
