# Construction contact and physical movement verification

The construction changes keep normal costs, starting resources, difficulty profiles,
unit radii, build times and match limits. Worker construction requires a real route
and contact outside the target's collision clearance. Existing structure overlap
can recover only through continuously outward motion. Resources, walls, cliffs and
new physical bodies remain impassable; enemy crushing keeps its existing exemption.

Contact planning uses one shared-grid search for up to 32 valid contact candidates.
Exact clearance-circle intersections supply candidates for narrow contact arcs.
Only an existing structure clearance overlap with no straight seed can use a bounded
25×25-cell initial escape prefix. Every prefix edge is checked, and its actual
waypoints remain in the returned route. Direct contact legs validate terrain and
exact footprints before aggregate raster cells can reject a geometrically valid
endpoint. Construction runs after ordinary movement and separation.

Actual movement validates each full static-body segment. Unsafe local steering can
fall back to a safe intended step. Constructors retain strict physical peer checks;
ordinary separation can use safe components of a blocked push without increasing
its speed cap. Constructor arrival tolerance leaves the required contact clearance
inside the existing contact ring. Production and the normal paid Refinery's bonus
Worker use the same occupied-footprint spawn search, including resources and walls.
A blocked bonus remains entitled until a safe spawn is available.

The recorded failures were distinct:

- TechDivide case0684 repeatedly spent its unchanged 180-second wall budget searching
  for contact from an existing PowerReactor overlap. The original trace reached
  435.115 simulation seconds; an eight-second CPU profile showed repeated contact
  searches and vector allocation. Living structure counts did not establish completion.
- A full-schedule Human0/Human1/AI2 fixture caught local steering entering a resource
  body at 19.4667 seconds: clearance 1.2m, actual distance 1.169674m. Its safe intended
  segment remained outside the body.
- A legitimate TechLab foundation surrounded its builder. A curved, continuously
  outward route existed while every straight contact leg was blocked.
- A Factory padding deficit exposed a narrow valid contact arc missed by uniform
  ring sampling and rasterized goal rejection.
- A constructor/harvester pair could not separate near existing buildings using only
  the blocked aggregate push. A safe component released the pair.
- The alternate-start match's bonus Worker was first observed inside a resource at
  12.6667 seconds, distance 0.9568214m versus a 1.2m physical clearance. The original
  unchecked Refinery offset caused that birth; it was not a trained-worker payment.

Retained RED logs include `logs/constructor-body-red-11.log`,
`logs/constructor-body-curved-red-18-tech.log`,
`logs/constructor-body-separation-red-22-tech.log`, and
`logs/constructor-body-green-red-23-spawn.log`. Numeric exit files accompany them.
Earlier unsuccessful fixes remain in the numbered logs rather than being reported
as successful verification.

GREEN26 passed all 14 geometry/contact/separation regressions and all seven AI
regressions. The unchanged BigArena Chaos Easy/Beginner repair-defense case ended
with team0 winning at 236.8216s; alternate PlainAndSimple Alliance Easy/Beginner
starts1/3 ended with team0 winning at 192.1788s. The 45-second controller fixture
kept Human1 resources and orders untouched while AI2 completed a paid Factory and
trained armed units. Paid technology completion and advanced armed production passed.

After splitting the pure planner module and correcting a blocked-route fixture to
use actual hard TerrainWall circles, full27 passed **219 tests, 0 failed, 1 ignored**
in 163.80 seconds. The wait fixture retains its stationary position, unchanged work,
no Move/Path, live ConstructOrder, and later completion assertions. Logs and persisted
exits are `logs/constructor-full-27-*`; the pipeline result is
`target/constructor-full-27-result.json`.

The four native binaries were sealed in `target/construction-nav-shared-20261010`.
The audit SHA256 is `fad4846516f6ae6f5a879ab5309d90b01f24ac561d074d297c774aac0a2f353f`;
`pin.json` records Rust/Cargo versions, every Rust/Cargo input and all 338 runtime
asset hashes. The native build exited0 in3m33s.

The exact previously failing case0684 then passed both trace and no-trace runs with
unchanged **600s simulation /180s wall limits**. Both processes exited0 and their raw
result JSONs are exactly equal: team1 won at180.542938 simulation seconds after5417
updates, with normal32/16 resources and the original spawn0/2 configuration. Actual
child wall times were19.91s and26.90s, including startup/shutdown. Trace final state
explicitly shows no pending foundations. Raw results, process metadata, JSONL and
independent comparison are in `target/tech-divide-construction-fixed-pair-05`.

These results establish this construction safety/contact regression and this exact
bounded match's termination. They do not establish all-map balance, all-start
completion, or general performance. The other broad-start failures remain separate
verification work.
