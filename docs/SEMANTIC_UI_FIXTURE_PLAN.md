# Semantic icon UI fixture

`capture semantic-ui <new-output> <width> <height>` renders the remaining10
integrated semantic icons using actual shared command/support-card builders,
icon mappings, hotkeys and tooltip definitions. Native-only helpers live in
`src/capture_api/semantic_ui.rs` and `src/bin/capture/semantic_ui.rs`.

The screen is visibly labeled “UI visual fixture — no gameplay acceptance”.
It has no units, production, combat, match progression or injected effects.
One isolated presentation economy slot supports the existing tooltip reader;
card visibility is a fixture setting, not proof of paid unlocking. Shared
support tooltips correctly report missing technology. The actual virtual cursor
hovers each card, and the observer requires matching hovered component identity
before recording the shared tooltip text. The updater runs once after hover,
without duplicate schedule registration.

## Verified evidence

Fixture07 passes at1280×720 and900×650 with distinct PIDs3807950/3808272,
child0 and valid observations. Capture SHA256:
`eeee95ce220c4f3df409b73ca192c6324465760ef90b3dcaa5b0ecaf0f892e8a`.
All22 overview/tooltip PNGs were individually inspected: icons are distinct,
text is readable, tooltip identity matches and no viewport clipping occurs.

Remaining10 covered: GuardArea, HoldPosition, DeployMode, Repair, ChronoRelay,
NaniteRepairSwarm, OrbitalStrike, Paradrop, StrategicMissile and WeatherStorm.
Prior real native/menu captures cover RadarSweep, EmpPulse, ShieldOverdrive,
RallyPoint, AttackMove, SellStructure, Patrol, Scatter, disabled StopCommand
and TechLocked. HQ AttackMove was initially mislabeled Repair; review03 was
corrected before accepting coverage.

`logs/semantic-ui-{1280,900}-07-result.json` and
`screenshots/steam-quality/semantic-ui-{1280,900}-07/observations.json` retain
actual child/executable identities, shared mapped paths and full hover strings.
`semantic-icons-native-review-07.json` in the same screenshot root nests those
results and the20-asset integration records. Failures05 (duplicate updater)
and06 (missing presentation slot) remain preserved under their original paths.

This closes glyph visual coverage20/20 only. It does not prove paid unlocking,
targeting, cooldowns, combat/support effects, campaign acceptance, store media
or final distribution readiness. Existing bounded F1/F5 and mine/order proofs
remain separate. No asset bytes or gameplay core changed for this fixture.
