# Current native save continuity

Schema 3 normal-input save and separate-process mouse Load passed for harvesting and two queued moves, paid mine deployment, and F1 Radar Sweep/F5 Shield Overdrive. Each flow used a fresh isolated profile, distinct process IDs, unchanged save bytes and the current 332 runtime assets. Exact executables, process IDs, hashes and raw observations are pinned in [the review](../art/save/current-native-continuity-review.json).

The mine caused exactly 4 damage to the observed harvesting Worker after restore. The support effects restored their target, remaining timers and cooldowns, then expired normally. These results cover only the listed states.

The native helper now checks actual viewport dimensions and real HUD exclusion zones instead of a fixed 900px limit. An effects failure was traced to JSON decimal representation of the same f32 world coordinate; the comparison now uses exact f32 coordinates. Entity remapping, shield strength, timer tolerances, cooldowns and expiry assertions remain unchanged. Earlier failed runs are preserved in the review.

Normal-input midfade weather, active paid construction and queued production continuity remain pending on the current core. Broader statuses/support powers and the final packaged executable require separate evidence.
