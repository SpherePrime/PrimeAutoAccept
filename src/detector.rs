pub struct Frame {
    pub width: usize,
    pub height: usize,
    pub bgra: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

const SEARCH_LEFT: f32 = 0.25;
const SEARCH_RIGHT: f32 = 0.75;
const SEARCH_TOP: f32 = 0.20;
const SEARCH_BOTTOM: f32 = 0.85;
const CENTER_LEFT: f32 = 0.30;
const CENTER_RIGHT: f32 = 0.70;
const MIN_WIDTH_RATIO: f32 = 0.05;
const MIN_HEIGHT_RATIO: f32 = 0.025;
const MIN_ASPECT: f32 = 1.8;
const MAX_ASPECT: f32 = 9.0;
const MIN_FILL: f32 = 0.6;
const TARGET_SAMPLES: usize = 480;

impl Frame {
    fn rgb(&self, x: usize, y: usize) -> (u8, u8, u8) {
        let offset = (y * self.width + x) * 4;
        (
            self.bgra[offset + 2],
            self.bgra[offset + 1],
            self.bgra[offset],
        )
    }
}

pub fn is_accept_green(red: u8, green: u8, blue: u8) -> bool {
    let (r, g, b) = (red as f32 / 255.0, green as f32 / 255.0, blue as f32 / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    if max < 0.38 || delta / max < 0.42 || max != g {
        return false;
    }
    let hue = 60.0 * (((b - r) / delta) + 2.0);
    (85.0..=160.0).contains(&hue)
}

struct Component {
    min_x: usize,
    max_x: usize,
    min_y: usize,
    max_y: usize,
    count: usize,
}

pub fn find_accept_button(frame: &Frame) -> Option<Point> {
    if frame.width < 64 || frame.height < 64 {
        return None;
    }
    let step = (frame.width / TARGET_SAMPLES).max(1);
    let left = (frame.width as f32 * SEARCH_LEFT) as usize;
    let right = (frame.width as f32 * SEARCH_RIGHT) as usize;
    let top = (frame.height as f32 * SEARCH_TOP) as usize;
    let bottom = (frame.height as f32 * SEARCH_BOTTOM) as usize;
    let grid_width = (right - left) / step;
    let grid_height = (bottom - top) / step;

    let mut mask = vec![false; grid_width * grid_height];
    for gy in 0..grid_height {
        for gx in 0..grid_width {
            let (r, g, b) = frame.rgb(left + gx * step, top + gy * step);
            mask[gy * grid_width + gx] = is_accept_green(r, g, b);
        }
    }

    let mut visited = vec![false; mask.len()];
    let mut best: Option<Component> = None;
    let mut queue = Vec::new();
    for start in 0..mask.len() {
        if !mask[start] || visited[start] {
            continue;
        }
        visited[start] = true;
        queue.push(start);
        let mut component = Component {
            min_x: usize::MAX,
            max_x: 0,
            min_y: usize::MAX,
            max_y: 0,
            count: 0,
        };
        while let Some(index) = queue.pop() {
            let (x, y) = (index % grid_width, index / grid_width);
            component.count += 1;
            component.min_x = component.min_x.min(x);
            component.max_x = component.max_x.max(x);
            component.min_y = component.min_y.min(y);
            component.max_y = component.max_y.max(y);
            let neighbours = [
                (x > 0).then(|| index - 1),
                (x + 1 < grid_width).then(|| index + 1),
                (y > 0).then(|| index - grid_width),
                (y + 1 < grid_height).then(|| index + grid_width),
            ];
            for next in neighbours.into_iter().flatten() {
                if mask[next] && !visited[next] {
                    visited[next] = true;
                    queue.push(next);
                }
            }
        }
        if looks_like_button(&component, frame, step, left)
            && best.as_ref().is_none_or(|b| component.count > b.count)
        {
            best = Some(component);
        }
    }

    best.map(|c| Point {
        x: (left + (c.min_x + c.max_x) * step / 2) as i32,
        y: (top + (c.min_y + c.max_y) * step / 2) as i32,
    })
}

fn looks_like_button(component: &Component, frame: &Frame, step: usize, left: usize) -> bool {
    let box_width = (component.max_x - component.min_x + 1) as f32;
    let box_height = (component.max_y - component.min_y + 1) as f32;
    let width_px = box_width * step as f32;
    let height_px = box_height * step as f32;
    let aspect = box_width / box_height;
    let fill = component.count as f32 / (box_width * box_height);
    let center_x = left as f32 + (component.min_x + component.max_x) as f32 * step as f32 / 2.0;
    width_px >= frame.width as f32 * MIN_WIDTH_RATIO
        && height_px >= frame.height as f32 * MIN_HEIGHT_RATIO
        && (MIN_ASPECT..=MAX_ASPECT).contains(&aspect)
        && fill >= MIN_FILL
        && center_x >= frame.width as f32 * CENTER_LEFT
        && center_x <= frame.width as f32 * CENTER_RIGHT
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame_with(fill: impl Fn(usize, usize) -> (u8, u8, u8)) -> Frame {
        let (width, height) = (1280, 720);
        let mut bgra = vec![0u8; width * height * 4];
        for y in 0..height {
            for x in 0..width {
                let (r, g, b) = fill(x, y);
                let offset = (y * width + x) * 4;
                bgra[offset] = b;
                bgra[offset + 1] = g;
                bgra[offset + 2] = r;
                bgra[offset + 3] = 255;
            }
        }
        Frame { width, height, bgra }
    }

    #[test]
    fn classifies_colours() {
        assert!(is_accept_green(54, 183, 82));
        assert!(is_accept_green(76, 175, 80));
        assert!(!is_accept_green(40, 60, 45));
        assert!(!is_accept_green(200, 50, 50));
        assert!(!is_accept_green(230, 230, 230));
    }

    #[test]
    fn finds_centered_button() {
        let frame = frame_with(|x, y| {
            if (520..760).contains(&x) && (330..400).contains(&y) {
                (54, 183, 82)
            } else {
                (20, 24, 30)
            }
        });
        let point = find_accept_button(&frame).expect("button found");
        assert!((point.x - 640).abs() < 8, "{point:?}");
        assert!((point.y - 365).abs() < 8, "{point:?}");
    }

    #[test]
    fn ignores_large_green_field() {
        let frame = frame_with(|x, y| {
            if (x + y) % 7 == 0 {
                (30, 30, 30)
            } else {
                (60, 160, 70)
            }
        });
        assert!(find_accept_button(&frame).is_none());
    }

    #[test]
    fn ignores_button_in_corner() {
        let frame = frame_with(|x, y| {
            if (1000..1250).contains(&x) && (640..700).contains(&y) {
                (54, 183, 82)
            } else {
                (20, 24, 30)
            }
        });
        assert!(find_accept_button(&frame).is_none());
    }
}
