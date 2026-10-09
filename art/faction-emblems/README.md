# Original faction emblem candidates

Alliance uses a winged star shield, Demon a horned flame crest, and Chaos a
split hexagonal rift. All geometry is authored in `generate.py`; there are no
imported images, fonts or third-party symbol inputs. Source SVGs use the
repository’s MIT/Apache-2.0 terms; no rights claim is made for the legacy images.

Generate into a new directory with `python3 art/faction-emblems/generate.py
<destination>`. resvg0.48.1 renders128px RGBA8 PNGs. Two fresh runs reproduce all
three PNGs byte-identically; alpha checks confirm transparent and opaque pixels.
The review sheet shows16/24/32px samples on light and dark backgrounds.

These are staged candidates. Current runtime emblems remain unchanged until
actual skirmish/lobby menu inspection passes. Manifest and verification files
retain exact source, tool and output hashes.
