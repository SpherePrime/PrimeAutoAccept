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

pub struct Area {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

pub const SEARCH_AREA: Area = Area {
    left: 0.25,
    top: 0.20,
    right: 0.75,
    bottom: 0.85,
};

const CENTER_MARGIN: f32 = 0.1;
const MIN_WIDTH_RATIO: f32 = 0.1;
const MIN_HEIGHT_RATIO: f32 = 0.038;
const MIN_ASPECT: f32 = 1.8;
const MAX_ASPECT: f32 = 9.0;
const MIN_FILL: f32 = 0.6;

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
    let (width, height) = (frame.width, frame.height);
    if width < 32 || height < 32 || frame.bgra.len() < width * height * 4 {
        return None;
    }
    let mask: Vec<bool> = (0..width * height)
        .map(|index| {
            let (r, g, b) = frame.rgb(index % width, index / width);
            is_accept_green(r, g, b)
        })
        .collect();

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
            let (x, y) = (index % width, index / width);
            component.count += 1;
            component.min_x = component.min_x.min(x);
            component.max_x = component.max_x.max(x);
            component.min_y = component.min_y.min(y);
            component.max_y = component.max_y.max(y);
            let neighbours = [
                (x > 0).then(|| index - 1),
                (x + 1 < width).then(|| index + 1),
                (y > 0).then(|| index - width),
                (y + 1 < height).then(|| index + width),
            ];
            for next in neighbours.into_iter().flatten() {
                if mask[next] && !visited[next] {
                    visited[next] = true;
                    queue.push(next);
                }
            }
        }
        if looks_like_button(&component, width, height)
            && best.as_ref().is_none_or(|b| component.count > b.count)
        {
            best = Some(component);
        }
    }

    best.map(|c| Point {
        x: ((c.min_x + c.max_x) / 2) as i32,
        y: ((c.min_y + c.max_y) / 2) as i32,
    })
}

fn looks_like_button(component: &Component, width: usize, height: usize) -> bool {
    let box_width = (component.max_x - component.min_x + 1) as f32;
    let box_height = (component.max_y - component.min_y + 1) as f32;
    let aspect = box_width / box_height;
    let fill = component.count as f32 / (box_width * box_height);
    let center_x = (component.min_x + component.max_x) as f32 / 2.0 / width as f32;
    box_width >= width as f32 * MIN_WIDTH_RATIO
        && box_height >= height as f32 * MIN_HEIGHT_RATIO
        && (MIN_ASPECT..=MAX_ASPECT).contains(&aspect)
        && fill >= MIN_FILL
        && (CENTER_MARGIN..=1.0 - CENTER_MARGIN).contains(&center_x)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame_with(fill: impl Fn(usize, usize) -> (u8, u8, u8)) -> Frame {
        let (width, height) = (320, 234);
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
            if (120..200).contains(&x) && (100..124).contains(&y) {
                (54, 183, 82)
            } else {
                (20, 24, 30)
            }
        });
        let point = find_accept_button(&frame).expect("button found");
        assert!((point.x - 160).abs() < 3, "{point:?}");
        assert!((point.y - 112).abs() < 3, "{point:?}");
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
    fn ignores_button_at_the_edge() {
        let frame = frame_with(|x, y| {
            if (272..320).contains(&x) && (200..222).contains(&y) {
                (54, 183, 82)
            } else {
                (20, 24, 30)
            }
        });
        assert!(find_accept_button(&frame).is_none());
    }
}
