import unittest
from benchmark import plan


class PlanTests(unittest.TestCase):
    def test_matrix_has_three_alternating_pairs_for_each_condition(self):
        runs = plan()
        self.assertEqual(len(runs), 24)
        for start in range(0, 24, 6):
            group = runs[start:start + 6]
            self.assertEqual([r['backend'] for r in group],
                             ['wgpu', 'miniquad', 'miniquad', 'wgpu', 'wgpu', 'miniquad'])
            self.assertEqual(len({(r['height'], r['dummies']) for r in group}), 1)
            self.assertEqual([r['repeat'] for r in group], [1, 1, 2, 2, 3, 3])
        self.assertEqual({(r['width'], r['height'], r['dummies']) for r in runs},
                         {(1280, 720, 5), (1280, 720, 20), (1920, 1080, 5), (1920, 1080, 20)})

class FilterTests(unittest.TestCase):
    def test_single_condition_can_be_repeated_without_overwriting_other_trials(self):
        runs = plan(height=1080, dummies=20)
        self.assertEqual(len(runs), 6)
        self.assertTrue(all(r['height'] == 1080 and r['dummies'] == 20 for r in runs))


if __name__ == '__main__':
    unittest.main()
