# Glyph mapping (Kenney Input Prompts 1.5)

Status: **proposal** for T1, when the frame shows prompts. Nothing uses these files yet.

**Basis:** the real file names in `assets/glyphs/kenney-input-prompts/`, which is Kenney's
`Vector` (SVG) folder of each set, unchanged; only the set folders were renamed (`Xbox Series` →
`xbox-series`, `Keyboard & Mouse` → `keyboard-mouse`, …). The WPF `Assets/Glyphs` folder was
**not** in the attached ZIPs, so this mapping could not be checked against the files approved
for WPF. Please compare it with WPF, or attach that folder.

Sets:
- **Xbox plain / monochrome:** the files without `_color_`.
- **PlayStation coloured:** the `_color_` face buttons.
- **Generic:** a fallback for pads of unknown type.
- **Keyboard:** keys for the keyboard actions.

Kenney's `_outline` variants exist for most files (for example for a pressed or disabled look).

| AIONE control (core name) | Xbox (plain) | PlayStation (coloured) | Generic fallback |
|---|---|---|---|
| A | `xbox_button_a.svg` | `playstation_button_color_cross.svg` | `generic_button_circle.svg` |
| B | `xbox_button_b.svg` | `playstation_button_color_circle.svg` | `generic_button_circle.svg` |
| X | `xbox_button_x.svg` | `playstation_button_color_square.svg` | `generic_button_square.svg` |
| Y | `xbox_button_y.svg` | `playstation_button_color_triangle.svg` | `generic_button_square.svg` |
| LB / RB | `xbox_lb.svg` / `xbox_rb.svg` | `playstation_trigger_l1.svg` / `playstation_trigger_r1.svg` | `generic_button_trigger_a.svg` |
| LT / RT | `xbox_lt.svg` / `xbox_rt.svg` | `playstation_trigger_l2.svg` / `playstation_trigger_r2.svg` | `generic_button_trigger_b.svg` |
| View | `xbox_button_view.svg` | `playstation5_button_create.svg` | `generic_button.svg` |
| Menu | `xbox_button_menu.svg` | `playstation5_button_options.svg` | `generic_button.svg` |
| Guide | `xbox_guide.svg` | — (the pack has no PS button glyph) | `generic_button_circle.svg` |
| LS / RS (click) | `xbox_ls.svg` / `xbox_rs.svg` | `playstation_button_l3.svg` / `playstation_button_r3.svg` | `generic_stick_press.svg` |
| D-pad (any) | `xbox_dpad.svg` | `playstation_dpad.svg` | `generic_stick.svg` |
| D-pad up / down / left / right | `xbox_dpad_up.svg` … `xbox_dpad_right.svg` | `playstation_dpad_up.svg` … | `generic_stick_up.svg` … |
| Left stick (any / a direction) | `xbox_stick_l.svg`, `xbox_stick_l_up.svg` … | `playstation_stick_l.svg`, `…_up.svg` … | `generic_stick.svg` … |
| Right stick (pointer) | `xbox_stick_r.svg` | `playstation_stick_r.svg` | `generic_stick.svg` |

| Keyboard action | File |
|---|---|
| Select (A) | `keyboard_enter.svg` (also `keyboard_space.svg`) |
| Back (B) | `keyboard_escape.svg` (also `keyboard_backspace.svg`) |
| Move | `keyboard_arrow_up.svg`, `…_down.svg`, `…_left.svg`, `…_right.svg` |
| Tabs (LB / RB) | `keyboard_q.svg` / `keyboard_e.svg` (to be decided in T1) |
| Restart (T0 only) | `keyboard_ctrl.svg` + `keyboard_shift.svg` + `keyboard_r.svg` |
| Mouse | `mouse_left.svg`, `mouse_right.svg`, `mouse_scroll.svg` |

Licence: CC0 (`assets/glyphs/kenney-input-prompts/License.txt`).
