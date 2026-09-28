import unittest
from summarize import summarize


def report(fps=60.):
    return {
        'comparison_valid': True, 'completed': True, 'release': True,
        'scene': 'm0-cubes-v1', 'backend': 'test GPU',
        'options': dict(width=1280, height=720, dummies=5, warmup=5., seconds=30., vsync=True, input=False, window_system='auto'),
        'display_pixels_initial': [1920, 1080], 'display_pixels_final': [1920, 1080],
        'frame_statistics': dict(average_fps=fps, one_percent_low_fps=55., p99_ms=18.),
        'rss_sampled_max_bytes': 1048576, 'startup_to_first_submit_return_ms': 100.,
    }


class SummaryTests(unittest.TestCase):
    def test_median_and_reject_invalid_trial(self):
        reports = [report(50.), report(60.), report(100.)]
        self.assertEqual(summarize(reports)['average_fps'], 60.)
        reports[1]['comparison_valid'] = False
        with self.assertRaises(ValueError):
            summarize(reports)

    def test_mismatched_display_or_options_are_rejected(self):
        for section, key, value in [('options', 'height', 1080), ('options', 'vsync', False)]:
            reports = [report(), report(), report()]
            reports[1][section][key] = value
            with self.assertRaises(ValueError):
                summarize(reports)
        reports = [report(), report(), report()]
        reports[1]['display_pixels_final'] = [1280, 720]
        with self.assertRaises(ValueError):
            summarize(reports)


if __name__ == '__main__':
    unittest.main()
