//! Fixed-width ASCII golf vignettes (SSH-safe, no wide unicode).

/// Inner width matches the main banner padding.
pub const SCENE_WIDTH: usize = 40;

/// Each scene is 3 lines; padded to SCENE_WIDTH at render time.
pub const GOLF_SCENES: &[&[&str]] = &[
    &[
        "           .   ~    ~    .          ",
        "             /|\\   flag             ",
        "    o-----> /_|_\\_______/~\\___      ",
    ],
    &[
        "      ~  ~       ___                ",
        "                /|_\\  <- green      ",
        "    Tee  o====='  .  ball in play   ",
    ],
    &[
        "          ________                  ",
        "         /|_\\____/\\  cart on path   ",
        "    ====O========O====   ~~  ~      ",
    ],
    &[
        "       *     .      *               ",
        "          \\|/                       ",
        "    ~~~~~/|\\~~~~~~ bunker splash    ",
    ],
    &[
        "               /|                   ",
        "    fairway __/_|\\__  pin ahead     ",
        "    .........o.........             ",
    ],
];

fn pad_scene_line(s: &str) -> String {
    let mut out: String = s.chars().take(SCENE_WIDTH).collect();
    while out.chars().count() < SCENE_WIDTH {
        out.push(' ');
    }
    out
}

/// Return the 3 padded lines for `idx`.
pub fn scene_lines(idx: usize) -> [String; 3] {
    let scene = GOLF_SCENES[idx % GOLF_SCENES.len()];
    [
        pad_scene_line(scene[0]),
        pad_scene_line(scene[1]),
        pad_scene_line(scene[2]),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scenes_fixed_width() {
        assert!(!GOLF_SCENES.is_empty());
        for i in 0..GOLF_SCENES.len() {
            for line in scene_lines(i) {
                assert_eq!(line.chars().count(), SCENE_WIDTH);
            }
        }
    }
}
