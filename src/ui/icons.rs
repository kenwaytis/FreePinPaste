use gpui::{prelude::*, *};

#[derive(Clone, Copy)]
pub enum Icon {
    Shape,
    Line,
    Pen,
    Text,
    Eraser,
    Ocr,
    Undo,
    Redo,
    Close,
    Pin,
    Save,
    Copy,
}

pub fn icon(icon: Icon, color: u32) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            let paths: Vec<Vec<(f32, f32)>> = match icon {
                Icon::Shape => vec![
                    vec![(3., 4.), (17., 4.), (17., 16.), (3., 16.), (3., 4.)],
                    (0..=24)
                        .map(|i| {
                            let a = i as f32 * std::f32::consts::TAU / 24.;
                            (16. + a.cos() * 5., 16. + a.sin() * 4.)
                        })
                        .collect(),
                ],
                Icon::Line => vec![
                    vec![(3., 19.), (10., 11.), (14., 14.), (21., 5.)],
                    vec![(15., 5.), (21., 5.), (21., 11.)],
                ],
                Icon::Pen => vec![
                    vec![
                        (4., 20.),
                        (6., 14.),
                        (17., 3.),
                        (21., 7.),
                        (10., 18.),
                        (4., 20.),
                    ],
                    vec![(15., 5.), (19., 9.)],
                ],
                Icon::Text => vec![
                    vec![(4., 5.), (20., 5.)],
                    vec![(12., 5.), (12., 21.)],
                    vec![(8., 21.), (16., 21.)],
                ],
                Icon::Eraser => vec![
                    vec![
                        (3., 15.),
                        (13., 4.),
                        (21., 11.),
                        (13., 20.),
                        (8., 20.),
                        (3., 15.),
                    ],
                    vec![(8., 10.), (16., 17.)],
                    vec![(13., 20.), (22., 20.)],
                ],
                Icon::Ocr => vec![
                    vec![(3., 9.), (3., 3.), (9., 3.)],
                    vec![(15., 3.), (21., 3.), (21., 9.)],
                    vec![(3., 15.), (3., 21.), (9., 21.)],
                    vec![(15., 21.), (21., 21.), (21., 15.)],
                    vec![(8., 16.), (12., 7.), (16., 16.)],
                    vec![(10., 13.), (14., 13.)],
                ],
                Icon::Undo => vec![
                    vec![(9., 5.), (3., 11.), (9., 16.)],
                    vec![(3., 11.), (15., 11.), (19., 13.), (20., 17.), (19., 21.)],
                ],
                Icon::Redo => vec![
                    vec![(15., 5.), (21., 11.), (15., 16.)],
                    vec![(21., 11.), (9., 11.), (5., 13.), (4., 17.), (5., 21.)],
                ],
                Icon::Close => vec![vec![(5., 5.), (19., 19.)], vec![(19., 5.), (5., 19.)]],
                Icon::Pin => vec![
                    vec![
                        (14., 3.),
                        (21., 10.),
                        (17., 10.),
                        (13., 15.),
                        (13., 19.),
                        (5., 11.),
                        (9., 11.),
                        (14., 6.),
                        (14., 3.),
                    ],
                    vec![(9., 15.), (3., 21.)],
                ],
                Icon::Save => vec![
                    vec![
                        (4., 3.),
                        (18., 3.),
                        (21., 6.),
                        (21., 21.),
                        (3., 21.),
                        (3., 3.),
                        (4., 3.),
                    ],
                    vec![(7., 3.), (7., 10.), (17., 10.), (17., 3.)],
                    vec![(7., 21.), (7., 14.), (17., 14.), (17., 21.)],
                ],
                Icon::Copy => vec![
                    vec![(8., 8.), (21., 8.), (21., 21.), (8., 21.), (8., 8.)],
                    vec![(4., 16.), (3., 16.), (3., 3.), (16., 3.), (16., 5.)],
                ],
            };
            for points in paths {
                let mut path = PathBuilder::stroke(px(1.55));
                for (index, (x, y)) in points.into_iter().enumerate() {
                    let p = bounds.origin + point(px(x), px(y));
                    if index == 0 {
                        path.move_to(p);
                    } else {
                        path.line_to(p);
                    }
                }
                if let Ok(path) = path.build() {
                    window.paint_path(path, rgb(color));
                }
            }
        },
    )
    .w(px(24.))
    .h(px(24.))
}
