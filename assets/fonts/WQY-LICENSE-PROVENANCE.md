# WenQuanYi Micro Hei Notices

Both local font name tables identify **WenQuanYi Micro Hei, Version 0.2.0-beta**,
with these embedded copyright notices:

> Digitized data copyright © 2007, Google Corporation.
> Copyright © 2008-2009 WenQuanYi Board of Trustees (http://wenq.org/) and Qianqian Fang

The accompanying WQY-LICENSE-* texts are byte-for-byte copies from the upstream
0.2.0-beta source archive distributed by Debian:

https://deb.debian.org/debian/pool/main/f/fonts-wqy-microhei/fonts-wqy-microhei_0.2.0-beta.orig.tar.gz

Archive SHA-256: `2802ac8023aa36a66ea6e7445854e3a078d377ffff42169341bd237871f7213e`.
Its README declares Apache 2.0 or GPL v3 with the font-embedding exception in
Appendix B. AUTHORS preserves the upstream contributor and inherited-font notices.
Both license alternatives are retained; this document does not change their terms.

| File | SHA-256 |
| --- | --- |
| WQY-LICENSE-Apache2.txt | `1c19d8eb506a0e9d501d882fc9fe94eca99bc7b2f69f5e64fa79b7ec48cee54d` |
| WQY-LICENSE-GPLv3.txt | `8ceb4b9ee5adedde47b31e975c1d90c73ad27b6b165a1dcd80c7c545eb65b903` |
| WQY-LICENSE-README.txt | `9b1f9a3d90f88041387700498296fc89d6d600d0ebc9b6258c119dd192eaf6b9` |
| WQY-LICENSE-AUTHORS.txt | `e980452a04c9e9dd7d6f0d8f72dd4c960f312207d64b849b73347beb02b9c3fa` |

## Reproducible conversion and subsetting

The runtime `wqy-microhei-ui.ttf` now comes directly from face 0 (Micro Hei) of
that exact TTC, using fontTools **4.61.1**. Run `scripts/subset_font.sh` to recreate
it. The script verifies the archive and retained notices, extracts to ignored
`target/font-source`, retains original timestamps and subsets only requested
Unicode characters. It never reads or overwrites the user's ignored full TTF.

| Artifact | SHA-256 |
| --- | --- |
| Official TTC | `e4bca8df123ce01b104780f576ea1a58b9a5ff1662a91124b6d3180cb6c88212` |
| Converted face 0 TTF | `272435f569f3e7ffa7cea299d0b175346738b5922eccaedb168eadb63f8123da` |
| Runtime subset | `4b9451969be0c79fe523fe7132813ab1ad5bda36efab7929e5b596570e65ef7f` |

Two repeated regenerations produce identical runtime bytes. The cmap has 1,219
codepoints and covers full ASCII, current Rust literals/Unicode escapes, and
all English/Chinese (`en`, `zh_CN`) CSV catalog characters. Polish is not a product
locale and is not collected. All 616 non-ASCII catalog codepoints, including 南,
are verified; 负 and the entire previous subset's cmap also remain covered.
There are no missing requested CJK/catalog characters. The Latin/punctuation
safety ranges remain, and every previously shared character advance width is
unchanged. `target/font-source/subset-report.json` records verification;
`docs/asset-source-hashes.json` retains the source/tool/catalog/output hashes.

This is a documented modification of the upstream font, not a change to its
licenses. Keep the original notices and both supplied license alternatives with
any distributed subset. Rendering/layout still requires fresh runtime captures.
