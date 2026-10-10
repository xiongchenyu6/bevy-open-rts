# Battlefield outskirts review

The renderer adds two unselectable meshes outside the original map rectangle: a dim terrain apron and a narrow boundary rim. The cloned ground texture uses mirrored repetition; the original texture, fog rectangle, camera controls, supported tilt/zoom, navigation and gameplay bounds are unchanged. No gameplay tags are attached to either mesh.

Two geometry/component tests pass, and the shared library checkpoint passed 237 tests with one ignored. Native minimum tilt at 900×720 and maximum tilt at 1280×720 each completed 11 screenshots with actual arrow/edge/minimap/wheel/rotation inputs and ordinary paid Power/Radar construction. All 22 frames were reviewed: outside-map void and hard repeat seams are absent; unexplored in-map black fog remains. Exact capture hash, settings and portable review evidence are in [the review record](qa/camera-outskirts-native-review.json).

The earlier default HUD workflow also passed at 1920, 1280 and 900 widths. Its producer placement differed from the original void screenshot, so that comparison alone does not establish causality. A controlled 1920 corner view before/after mirrored sampling kept identical camera focus and fog counts and removed repetition seams. Raw local evidence remains under `target/apron-mirror-default-03-20261010` and `target/apron-camera-edges-05-20261010`; assets stayed byte-identical across each run.

These are bounded native presentation checks. They do not establish browser parity, performance, every map/zoom combination, or general release readiness. The helper observes real UI buttons and input; earlier failed runs are retained, including the corrected readonly OptionsMenuButton lookup error.
