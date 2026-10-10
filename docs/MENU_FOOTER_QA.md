# Skirmish setup footer and scrolling

The setup dialog now fits inside the client area. Its content scrolls while Start Match and Back remain visible. The content owns the existing menu scroll component; the footer cannot shrink. Scroll input clamps the requested offset to computed content overflow, so one upward wheel notch responds immediately after repeated downward scrolling.

The prior candidate exposed a real reverse-scroll regression: after 25 downward inputs, one upward input produced byte-identical screenshots at both 1280 and 900 widths. The corrected native binary passes normal input at both widths with a 696px client area, plus the game's normal Fullscreen button at 720px. The footer remains visible, reverse input restores the upper content, and Back returns to the main menu. The game normally closes with exit 0 in 136.82 seconds, inside the original 235/240-second driver/runner limits.

Earlier runner failures are retained: its window filter rejected a real unnamed-app-id Wayland view, and an asynchronous screenshot/geometry race falsely rejected a successful fullscreen transition. These are not passing attempts. The corrected runner identifies the actual Open RTS leaf and waits until the matching window tree and geometry are written.

[Bounded proof](qa/menu-footer-scroll-green-review.json) records binary identity, frame hashes, window geometry and prior RED evidence. This proves this native menu only; the optimized SDK package, browser, Windows and wider HUD still require verification.
