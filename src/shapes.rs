use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use std::f32::consts::{PI, TAU};

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ShapeKind {
    Line,
    Curve3,
    Curve4,
    Curve5,
    Circle,
    Capsule,
    Square,
    RoundedSquare,
    Triangle,
    RoundedTriangle,
    Pentagon,
    Hexagon,
    Diamond,
    Arrow,
    Star5,
    Star6,
    WordBalloon,
    ThoughtBalloon,
    XMark,
    CheckMark,
    LightningBolt,
    Heart,
}
impl ShapeKind {
    pub const LINES: [Self; 4] = [Self::Line, Self::Curve3, Self::Curve4, Self::Curve5];
    pub const SHAPES: [Self; 18] = [
        Self::Circle,
        Self::Capsule,
        Self::Square,
        Self::RoundedSquare,
        Self::Triangle,
        Self::RoundedTriangle,
        Self::Pentagon,
        Self::Hexagon,
        Self::Diamond,
        Self::Arrow,
        Self::Star5,
        Self::Star6,
        Self::WordBalloon,
        Self::ThoughtBalloon,
        Self::XMark,
        Self::CheckMark,
        Self::LightningBolt,
        Self::Heart,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Line => "Graphic Line",
            Self::Curve3 => "3 Point Curve",
            Self::Curve4 => "4 Point Curve",
            Self::Curve5 => "5 Point Curve",
            Self::Circle => "Circle",
            Self::Capsule => "Capsule",
            Self::Square => "Square",
            Self::RoundedSquare => "Rounded Square",
            Self::Triangle => "Triangle",
            Self::RoundedTriangle => "Rounded Triangle",
            Self::Pentagon => "Pentagon",
            Self::Hexagon => "Hexagon",
            Self::Diamond => "Diamond",
            Self::Arrow => "Arrow",
            Self::Star5 => "5 Pointed Star",
            Self::Star6 => "6 Pointed Star",
            Self::WordBalloon => "Word Balloon",
            Self::ThoughtBalloon => "Thought Balloon",
            Self::XMark => "X Mark",
            Self::CheckMark => "Check Mark",
            Self::LightningBolt => "Lightning Bolt",
            Self::Heart => "Heart",
        }
    }
}

pub fn picker(
    ctx: &egui::Context,
    open: &mut bool,
    selected: &mut Option<ShapeKind>,
) -> Option<ShapeKind> {
    let mut chosen = None;
    let mut close = false;
    egui::Window::new("2-D Shapes")
        .open(open)
        .collapsible(false)
        .resizable(false)
        .default_width(260.0)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .max_height(530.0)
                .show(ui, |ui| {
                    ui.heading("Lines and Curves");
                    chosen = choices(ui, &ShapeKind::LINES, selected, "curves").or(chosen);
                    ui.add_space(10.0);
                    ui.heading("2D Shapes");
                    chosen = choices(ui, &ShapeKind::SHAPES, selected, "shapes").or(chosen);
                });
            ui.separator();
            if let Some(shape) = selected {
                ui.label(format!("Selected: {}", shape.name()));
            }
            if ui.button("Close").clicked() {
                close = true;
            }
        });
    if close || chosen.is_some() {
        *open = false;
    }
    chosen
}

fn choices(
    ui: &mut egui::Ui,
    choices: &[ShapeKind],
    selected: &mut Option<ShapeKind>,
    id: &str,
) -> Option<ShapeKind> {
    let mut chosen = None;
    egui::Grid::new(id)
        .num_columns(6)
        .spacing([6.0, 6.0])
        .show(ui, |ui| {
            for (index, shape) in choices.iter().enumerate() {
                let response = ui.add(
                    egui::Button::new("")
                        .min_size(Vec2::splat(crate::toolbar::BUTTON_SIZE))
                        .selected(*selected == Some(*shape)),
                );
                let preview = Rect::from_center_size(response.rect.center(), Vec2::splat(20.0));
                let color = ui.style().interact(&response).fg_stroke.color;
                paint(ui.painter(), *shape, preview, color);
                let response = response.on_hover_text(shape.name());
                if response.clicked() {
                    *selected = Some(*shape);
                    chosen = Some(*shape);
                }
                if (index + 1) % 6 == 0 {
                    ui.end_row();
                }
            }
        });
    chosen
}

fn paint(painter: &egui::Painter, kind: ShapeKind, rect: Rect, color: Color32) {
    let stroke = Stroke::new(1.8_f32, color);
    for (points, closed) in outlines(kind, rect) {
        if kind == ShapeKind::Heart {
            // The heart's cusp makes egui's miter join extend far below the
            // icon. Round joins match the canvas renderer without a spike.
            for segment in points.windows(2) {
                painter.line_segment([segment[0], segment[1]], stroke);
            }
            if closed {
                painter.line_segment([*points.last().unwrap(), points[0]], stroke);
            }
            for point in points {
                painter.circle_filled(point, stroke.width / 2.0, color);
            }
            continue;
        }
        painter.add(if closed {
            egui::Shape::closed_line(points, stroke)
        } else {
            egui::Shape::line(points, stroke)
        });
    }
}

fn outlines(kind: ShapeKind, rect: Rect) -> Vec<(Vec<Pos2>, bool)> {
    let paths = std::cell::RefCell::new(Vec::new());
    let p = |x: f32, y: f32| rect.min + rect.size() * egui::vec2(x, y);
    let path = |points: Vec<Pos2>, closed: bool| paths.borrow_mut().push((points, closed));
    let circle = |center: Pos2, radius: Vec2| {
        path(
            (0..96)
                .map(|i| {
                    let angle = TAU * i as f32 / 96.0;
                    center + radius * egui::vec2(angle.cos(), angle.sin())
                })
                .collect(),
            true,
        );
    };
    let polygon = |n: usize, star: bool| -> Vec<Pos2> {
        let points: Vec<_> = (0..n)
            .map(|i| {
                let angle = -PI / 2.0 + TAU * i as f32 / n as f32;
                let radius = if star && i % 2 == 1 { 0.22 } else { 0.48 };
                p(0.5 + radius * angle.cos(), 0.5 + radius * angle.sin())
            })
            .collect();
        points
    };
    match kind {
        ShapeKind::Line => path(vec![p(0.05, 0.9), p(0.95, 0.1)], false),
        ShapeKind::Curve3 | ShapeKind::Curve4 | ShapeKind::Curve5 => {
            let controls = match kind {
                ShapeKind::Curve3 => vec![p(0.0, 0.9), p(0.5, -0.6), p(1.0, 0.9)],
                ShapeKind::Curve4 => vec![p(0.0, 0.9), p(0.3, -1.0), p(0.7, 2.0), p(1.0, 0.1)],
                _ => vec![
                    p(0.0, 0.9),
                    p(0.1, -0.9),
                    p(0.5, 1.8),
                    p(0.9, -0.9),
                    p(1.0, 0.9),
                ],
            };
            path(
                (0..=40)
                    .map(|i| {
                        let t = i as f32 / 40.0;
                        let mut points = controls.clone();
                        for len in (1..points.len()).rev() {
                            for j in 0..len {
                                points[j] = points[j].lerp(points[j + 1], t);
                            }
                        }
                        points[0]
                    })
                    .collect(),
                false,
            );
        }
        ShapeKind::Circle => {
            circle(rect.center(), rect.size() / 2.0);
        }
        ShapeKind::Square | ShapeKind::RoundedSquare | ShapeKind::Capsule => {
            let radius = match kind {
                ShapeKind::RoundedSquare => rect.width().min(rect.height()) * 0.18,
                ShapeKind::Capsule => rect.width().min(rect.height()) / 2.0,
                _ => 0.0,
            };
            let mut points = Vec::new();
            for (cx, cy, angle) in [
                (rect.right() - radius, rect.top() + radius, -PI / 2.0),
                (rect.right() - radius, rect.bottom() - radius, 0.0),
                (rect.left() + radius, rect.bottom() - radius, PI / 2.0),
                (rect.left() + radius, rect.top() + radius, PI),
            ] {
                for step in 0..=16 {
                    let t = angle + step as f32 / 16.0 * PI / 2.0;
                    points.push(egui::pos2(cx + radius * t.cos(), cy + radius * t.sin()));
                }
            }
            path(points, true);
        }
        ShapeKind::Triangle => path(vec![p(0.5, 0.0), p(1.0, 1.0), p(0.0, 1.0)], true),
        ShapeKind::RoundedTriangle => {
            let vertices = [p(0.5, 0.0), p(1.0, 1.0), p(0.0, 1.0)];
            let mut result = Vec::new();
            for i in 0..3 {
                let corner = vertices[i];
                let a = corner.lerp(vertices[(i + 2) % 3], 0.18);
                let b = corner.lerp(vertices[(i + 1) % 3], 0.18);
                for step in 0..=8 {
                    let t = step as f32 / 8.0;
                    result.push(a.lerp(corner, t).lerp(corner.lerp(b, t), t));
                }
            }
            path(result, true);
        }
        ShapeKind::Pentagon => path(polygon(5, false), true),
        ShapeKind::Hexagon => path(polygon(6, false), true),
        ShapeKind::Diamond => path(polygon(4, false), true),
        ShapeKind::Star5 => path(polygon(10, true), true),
        ShapeKind::Star6 => path(polygon(12, true), true),
        ShapeKind::Arrow => path(
            vec![
                p(0.0, 0.3),
                p(0.6, 0.3),
                p(0.6, 0.0),
                p(1.0, 0.5),
                p(0.6, 1.0),
                p(0.6, 0.7),
                p(0.0, 0.7),
            ],
            true,
        ),
        ShapeKind::WordBalloon => path(
            vec![
                p(0.05, 0.05),
                p(0.95, 0.05),
                p(0.95, 0.7),
                p(0.4, 0.7),
                p(0.15, 1.0),
                p(0.2, 0.7),
                p(0.05, 0.7),
            ],
            true,
        ),
        ShapeKind::ThoughtBalloon => {
            circle(p(0.18, 0.85), rect.size() * 0.027);
            circle(p(0.25, 0.765), rect.size() * 0.045);

            // Trace the pill and its attached bubble as one boundary, avoiding
            // an internal border where the largest bubble joins the body.
            let cap_center = egui::pos2(0.40, 0.30);
            let bubble_center = egui::pos2(0.32, 0.60);
            let cap_radius: f32 = 0.30;
            let bubble_radius: f32 = 0.105;
            let offset = bubble_center - cap_center;
            let distance = offset.length();
            let direction = offset / distance;
            let along =
                (cap_radius.powi(2) - bubble_radius.powi(2) + distance.powi(2)) / (2.0 * distance);
            let perpendicular = (cap_radius.powi(2) - along.powi(2)).sqrt();
            let intersection = cap_center
                + direction * along
                + egui::vec2(-direction.y, direction.x) * perpendicular;
            let bubble_end = (intersection.y - bubble_center.y)
                .atan2(intersection.x - bubble_center.x)
                .rem_euclid(TAU);
            let cap_start = (intersection.y - cap_center.y).atan2(intersection.x - cap_center.x);
            let mut points = vec![p(0.40, 0.0), p(0.70, 0.0)];
            for step in 0..=48 {
                let angle = -PI / 2.0 + PI * step as f32 / 48.0;
                points.push(p(
                    0.70 + cap_radius * angle.cos(),
                    0.30 + cap_radius * angle.sin(),
                ));
            }
            for step in 0..=48 {
                let angle = bubble_end * step as f32 / 48.0;
                points.push(p(
                    bubble_center.x + bubble_radius * angle.cos(),
                    bubble_center.y + bubble_radius * angle.sin(),
                ));
            }
            for step in 0..=48 {
                let angle = cap_start + (1.5 * PI - cap_start) * step as f32 / 48.0;
                points.push(p(
                    cap_center.x + cap_radius * angle.cos(),
                    cap_center.y + cap_radius * angle.sin(),
                ));
            }
            path(points, true);
        }
        ShapeKind::XMark => path(
            vec![
                p(0.1, 0.0),
                p(0.5, 0.38),
                p(0.9, 0.0),
                p(1.0, 0.1),
                p(0.62, 0.5),
                p(1.0, 0.9),
                p(0.9, 1.0),
                p(0.5, 0.62),
                p(0.1, 1.0),
                p(0.0, 0.9),
                p(0.38, 0.5),
                p(0.0, 0.1),
            ],
            true,
        ),
        ShapeKind::CheckMark => path(
            vec![
                p(0.0, 0.5),
                p(0.15, 0.36),
                p(0.36, 0.65),
                p(0.88, 0.0),
                p(1.0, 0.12),
                p(0.36, 1.0),
            ],
            true,
        ),
        ShapeKind::LightningBolt => path(
            vec![
                p(0.55, 0.0),
                p(0.15, 0.6),
                p(0.45, 0.6),
                p(0.35, 1.0),
                p(0.9, 0.35),
                p(0.6, 0.35),
            ],
            true,
        ),
        ShapeKind::Heart => path(
            (0..=100)
                .map(|i| {
                    let t = TAU * i as f32 / 100.0;
                    p(
                        0.5 + 0.03 * 16.0 * t.sin().powi(3),
                        0.46 - 0.03
                            * (13.0 * t.cos()
                                - 5.0 * (2.0 * t).cos()
                                - 2.0 * (3.0 * t).cos()
                                - (4.0 * t).cos()),
                    )
                })
                .collect(),
            true,
        ),
    }
    // Fit the actual geometry, including all components of a balloon, rather
    // than its construction circle or control points. Icons and raster output
    // share these bounds, so every selector encloses the visible shape tightly.
    let mut paths = paths.into_inner();
    let vertices: Vec<_> = paths
        .iter()
        .flat_map(|(points, _)| points.iter().copied())
        .collect();
    let bounds = Rect::from_points(&vertices);
    for (points, _) in &mut paths {
        for point in points {
            let fraction = (*point - bounds.min) / bounds.size();
            *point = rect.min + fraction * rect.size();
        }
    }
    paths
}

#[derive(Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Style {
    pub border_width: f32,
    pub border: Color32,
    pub fill: Color32,
    pub transparency: f32,
    pub transform: Transform,
    pub curve_points: Option<[Pos2; 5]>,
}
impl Default for Style {
    fn default() -> Self {
        Self {
            border_width: 3.0,
            border: Color32::BLACK,
            fill: Color32::WHITE,
            transparency: 0.0,
            transform: Transform::default(),
            curve_points: None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Transform {
    matrix: [f32; 4],
    source_aspect: Option<f32>,
}
impl Default for Transform {
    fn default() -> Self {
        Self {
            matrix: [1.0, 0.0, 0.0, 1.0],
            source_aspect: None,
        }
    }
}

fn transformed_paths(kind: ShapeKind, transform: Transform) -> Vec<(Vec<Pos2>, bool)> {
    let source = Rect::from_min_size(
        Pos2::ZERO,
        egui::vec2(transform.source_aspect.unwrap_or(1.0), 1.0),
    );
    let [a, b, c, d] = transform.matrix;
    outlines(kind, source)
        .into_iter()
        .map(|(points, closed)| {
            (
                points
                    .into_iter()
                    .map(|p| egui::pos2(a * p.x + b * p.y, c * p.x + d * p.y))
                    .collect(),
                closed,
            )
        })
        .collect()
}

fn path_bounds(paths: &[(Vec<Pos2>, bool)]) -> Rect {
    let points: Vec<_> = paths
        .iter()
        .flat_map(|(points, _)| points.iter().copied())
        .collect();
    Rect::from_points(&points)
}

fn oriented_outlines(kind: ShapeKind, transform: Transform, rect: Rect) -> Vec<(Vec<Pos2>, bool)> {
    if transform.source_aspect.is_none() {
        return outlines(kind, rect);
    }
    let mut paths = transformed_paths(kind, transform);
    let bounds = path_bounds(&paths);
    for (points, _) in &mut paths {
        for point in points {
            *point = rect.min + (*point - bounds.min) / bounds.size() * rect.size();
        }
    }
    paths
}

// Transform the currently displayed geometry, including any previous stretching.
// Expand the selector around its center rather than squeezing the rotated shape.
pub fn transform_shape(
    kind: ShapeKind,
    style: &mut Style,
    rect: &mut Rect,
    degrees: f32,
    horizontal: bool,
    vertical: bool,
) {
    if let Some(mut points) = style.curve_points {
        let count = if kind == ShapeKind::Curve4 { 4 } else { 5 };
        let padding = shape_padding(style, rect.size());
        let inner = rect.shrink(padding);
        let (sin, cos) = degrees.to_radians().sin_cos();
        for point in &mut points[..count] {
            let v = inner.min + point.to_vec2() * inner.size() - rect.center();
            *point = rect.center()
                + egui::vec2(
                    (cos * v.x - sin * v.y) * if horizontal { -1.0 } else { 1.0 },
                    (sin * v.x + cos * v.y) * if vertical { -1.0 } else { 1.0 },
                );
        }
        let bounds = Rect::from_points(&points[..count]);
        let bounds = Rect::from_center_size(bounds.center(), bounds.size().max(Vec2::splat(1.0)));
        *rect = bounds.expand(padding);
        for point in &mut points[..count] {
            *point = Pos2::ZERO + (*point - bounds.min) / bounds.size();
        }
        style.curve_points = Some(points);
        return;
    }
    if style.transform.source_aspect.is_none() {
        let padding = (style.border_width / 2.0 + 1.0).min(rect.width().min(rect.height()) / 4.0);
        style.transform.source_aspect =
            Some((rect.width() - 2.0 * padding) / (rect.height() - 2.0 * padding));
    }
    let bounds = path_bounds(&transformed_paths(kind, style.transform));
    let [a, b, c, d] = style.transform.matrix;
    let scale = rect.size() / bounds.size();
    let (sin, cos) = degrees.to_radians().sin_cos();
    let fx = if horizontal { -1.0 } else { 1.0 };
    let fy = if vertical { -1.0 } else { 1.0 };
    let mut matrix = [
        fx * (cos * scale.x * a - sin * scale.y * c),
        fx * (cos * scale.x * b - sin * scale.y * d),
        fy * (sin * scale.x * a + cos * scale.y * c),
        fy * (sin * scale.x * b + cos * scale.y * d),
    ];
    for value in &mut matrix {
        if value.abs() < 0.00001 {
            *value = 0.0;
        }
    }
    style.transform.matrix = matrix;
    let new_size = path_bounds(&transformed_paths(kind, style.transform)).size();
    let factor = (16384.0 / new_size.x).min(16384.0 / new_size.y).min(1.0);
    *rect = Rect::from_center_size(rect.center(), new_size * factor);
    let magnitude = matrix.iter().fold(0.0_f32, |m, v| m.max(v.abs()));
    for value in &mut style.transform.matrix {
        *value /= magnitude;
    }
}

fn shape_padding(style: &Style, size: Vec2) -> f32 {
    (style.border_width / 2.0 + 1.0).min(size.x.min(size.y) / 4.0)
}

fn bezier(controls: &[Pos2]) -> Vec<Pos2> {
    (0..=128)
        .map(|i| {
            let t = i as f32 / 128.0;
            let mut points = controls.to_vec();
            for len in (1..points.len()).rev() {
                for j in 0..len {
                    points[j] = points[j].lerp(points[j + 1], t);
                }
            }
            points[0]
        })
        .collect()
}

pub fn enable_curve_edit(kind: ShapeKind, style: &mut Style, rect: &mut Rect) {
    if style.curve_points.is_some() || !matches!(kind, ShapeKind::Curve4 | ShapeKind::Curve5) {
        return;
    }
    let controls = if kind == ShapeKind::Curve4 {
        vec![
            egui::pos2(0.0, 0.9),
            egui::pos2(0.3, -1.0),
            egui::pos2(0.7, 2.0),
            egui::pos2(1.0, 0.1),
        ]
    } else {
        vec![
            egui::pos2(0.0, 0.9),
            egui::pos2(0.1, -0.9),
            egui::pos2(0.5, 1.8),
            egui::pos2(0.9, -0.9),
            egui::pos2(1.0, 0.9),
        ]
    };
    let padding = shape_padding(style, rect.size());
    let target = rect.shrink(padding);
    let source = if let Some(aspect) = style.transform.source_aspect {
        Rect::from_min_size(Pos2::ZERO, egui::vec2(aspect, 1.0))
    } else {
        target
    };
    let mut controls: Vec<_> = controls
        .into_iter()
        .map(|p| source.min + p.to_vec2() * source.size())
        .collect();
    let curve_bounds = Rect::from_points(&bezier(&controls));
    for p in &mut controls {
        *p = source.min + (*p - curve_bounds.min) / curve_bounds.size() * source.size();
    }
    if style.transform.source_aspect.is_some() {
        let [a, b, c, d] = style.transform.matrix;
        for p in &mut controls {
            *p = egui::pos2(a * p.x + b * p.y, c * p.x + d * p.y);
        }
        let bounds = Rect::from_points(&bezier(&controls));
        for p in &mut controls {
            *p = target.min + (*p - bounds.min) / bounds.size() * target.size();
        }
    }
    let bounds = Rect::from_points(&controls);
    *rect = bounds.expand(padding);
    let mut points = [Pos2::ZERO; 5];
    for (dst, p) in points.iter_mut().zip(controls) {
        *dst = Pos2::ZERO + (p - bounds.min) / bounds.size();
    }
    style.curve_points = Some(points);
}

pub fn curve_editor(
    ui: &mut egui::Ui,
    kind: ShapeKind,
    style: &mut Style,
    object: &mut Rect,
    canvas: Rect,
    scale: f32,
) {
    let count = if kind == ShapeKind::Curve4 { 4 } else { 5 };
    let padding = shape_padding(style, object.size());
    let inner = object.shrink(padding);
    let Some(points) = &mut style.curve_points else {
        return;
    };
    let screen = |p: Pos2| canvas.min + (inner.min.to_vec2() + p.to_vec2() * inner.size()) * scale;
    ui.painter().add(egui::Shape::line(
        points[..count].iter().copied().map(screen).collect(),
        Stroke::new(1.0_f32, Color32::LIGHT_BLUE),
    ));
    let mut moved = None;
    for (index, point) in points[..count].iter().enumerate() {
        let center = screen(*point);
        let handle = Rect::from_center_size(center, Vec2::splat(10.0));
        ui.painter().rect_filled(handle, 0.0, Color32::GREEN);
        ui.painter().rect_stroke(
            handle,
            0.0,
            Stroke::new(1.0_f32, Color32::BLACK),
            egui::StrokeKind::Inside,
        );
        let response = ui.interact(
            handle.expand(3.0),
            ui.id().with(("curve_point", index)),
            egui::Sense::drag(),
        );
        if response.dragged_by(egui::PointerButton::Primary)
            && let Some(pointer) = response.interact_pointer_pos()
        {
            let position = (pointer - canvas.min) / scale;
            let image_size = canvas.size() / scale;
            moved = Some((
                index,
                egui::pos2(
                    position.x.clamp(0.0, image_size.x),
                    position.y.clamp(0.0, image_size.y),
                ),
            ));
        }
        response
            .on_hover_and_drag_cursor(egui::CursorIcon::Crosshair)
            .on_hover_text(if index == 0 || index == count - 1 {
                "Curve endpoint: drag to move"
            } else {
                "Control point: drag to shape the bend"
            });
    }
    if let Some((index, position)) = moved {
        move_curve_point(kind, style, object, index, position);
    }
}

pub(crate) fn move_curve_point(
    kind: ShapeKind,
    style: &mut Style,
    object: &mut Rect,
    index: usize,
    position: Pos2,
) {
    let count = if kind == ShapeKind::Curve4 { 4 } else { 5 };
    let inner = object.shrink(shape_padding(style, object.size()));
    let Some(points) = &mut style.curve_points else {
        return;
    };
    let mut world: Vec<_> = points[..count]
        .iter()
        .map(|p| inner.min + p.to_vec2() * inner.size())
        .collect();
    world[index] = position;
    let bounds = Rect::from_points(&world);
    let bounds = Rect::from_center_size(bounds.center(), bounds.size().max(Vec2::splat(1.0)));
    let padding = (style.border_width / 2.0 + 1.0).min(bounds.width().min(bounds.height()) / 2.0);
    *object = bounds.expand(padding);
    for (point, world) in points.iter_mut().zip(world) {
        *point = Pos2::ZERO + (world - bounds.min) / bounds.size();
    }
}

pub fn render(kind: ShapeKind, style: Style, width: u32, height: u32) -> image::RgbaImage {
    // Rasterize coverage independently, then give the stroke exclusive ownership
    // of its area. Painting a translucent stroke over a fill doubles the opacity.
    let mut fill_mask = tiny_skia::Pixmap::new(width, height).expect("nonzero shape dimensions");
    let mut border_mask = tiny_skia::Pixmap::new(width, height).expect("nonzero shape dimensions");
    let size = egui::vec2(width as f32, height as f32);
    let padding = (style.border_width / 2.0 + 1.0).min(size.x.min(size.y) / 4.0);
    let rect = Rect::from_min_max(
        egui::pos2(padding, padding),
        egui::pos2(size.x - padding, size.y - padding),
    );
    let mut coverage_paint = tiny_skia::Paint::default();
    coverage_paint.set_color_rgba8(255, 255, 255, 255);
    coverage_paint.anti_alias = true;
    let paths = if let Some(points) = style.curve_points {
        let count = if kind == ShapeKind::Curve4 { 4 } else { 5 };
        let controls: Vec<_> = points[..count]
            .iter()
            .map(|p| rect.min + p.to_vec2() * rect.size())
            .collect();
        vec![(bezier(&controls), false)]
    } else {
        oriented_outlines(kind, style.transform, rect)
    };
    for (points, closed) in paths {
        let mut builder = tiny_skia::PathBuilder::new();
        builder.move_to(points[0].x, points[0].y);
        for point in &points[1..] {
            builder.line_to(point.x, point.y);
        }
        if closed {
            builder.close();
        }
        let Some(path) = builder.finish() else {
            continue;
        };
        if closed {
            fill_mask.fill_path(
                &path,
                &coverage_paint,
                tiny_skia::FillRule::Winding,
                tiny_skia::Transform::identity(),
                None,
            );
        }
        if style.border_width > 0.0 {
            border_mask.stroke_path(
                &path,
                &coverage_paint,
                &tiny_skia::Stroke {
                    width: style.border_width,
                    line_cap: tiny_skia::LineCap::Round,
                    line_join: tiny_skia::LineJoin::Round,
                    ..Default::default()
                },
                tiny_skia::Transform::identity(),
                None,
            );
        }
    }
    let fill = style.fill.to_srgba_unmultiplied();
    let border = style.border.to_srgba_unmultiplied();
    image::RgbaImage::from_fn(width, height, |x, y| {
        let border_coverage = border_mask.pixel(x, y).unwrap().alpha() as u32;
        let fill_coverage =
            (fill_mask.pixel(x, y).unwrap().alpha() as u32).saturating_sub(border_coverage);
        let fill_weight = fill_coverage * fill[3] as u32;
        let border_weight = border_coverage * border[3] as u32;
        let total = fill_weight + border_weight;
        if total == 0 {
            return image::Rgba([0; 4]);
        }
        let alpha = (total as f32 / 255.0 * (1.0 - style.transparency / 100.0)).round() as u8;
        let channel = |i: usize| {
            ((fill[i] as u32 * fill_weight + border[i] as u32 * border_weight + total / 2) / total)
                as u8
        };
        image::Rgba([channel(0), channel(1), channel(2), alpha])
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn pixel_error(a: &image::Rgba<u8>, b: &image::Rgba<u8>) -> u8 {
        (0..4)
            .map(|i| {
                if i == 3 {
                    a[3].abs_diff(b[3])
                } else {
                    let left = (a[i] as u16 * a[3] as u16 / 255) as u8;
                    let right = (b[i] as u16 * b[3] as u16 / 255) as u8;
                    left.abs_diff(right)
                }
            })
            .max()
            .unwrap()
    }

    #[test]
    fn heart_icon_stroke_stays_inside_its_picker_button() {
        let ctx = egui::Context::default();
        let button = Rect::from_min_size(egui::pos2(20.0, 20.0), Vec2::splat(34.0));
        let preview = Rect::from_center_size(button.center(), Vec2::splat(20.0));
        let output = ctx.run(egui::RawInput::default(), |ctx| {
            paint(
                &ctx.layer_painter(egui::LayerId::background()),
                ShapeKind::Heart,
                preview,
                Color32::WHITE,
            );
        });
        let primitives = ctx.tessellate(output.shapes, output.pixels_per_point);
        let mut count = 0;
        for primitive in primitives {
            if let egui::epaint::Primitive::Mesh(mesh) = primitive.primitive {
                for vertex in mesh.vertices {
                    count += 1;
                    assert!(
                        button.contains(vertex.pos),
                        "Heart icon extends outside button: {:?}",
                        vertex.pos
                    );
                }
            }
        }
        assert!(count > 0);
    }

    #[test]
    fn curve_endpoints_move_independently_beyond_original_selector() {
        for kind in [ShapeKind::Curve4, ShapeKind::Curve5] {
            let mut style = Style::default();
            let mut rect = Rect::from_min_size(egui::pos2(200.0, 200.0), Vec2::splat(100.0));
            enable_curve_edit(kind, &mut style, &mut rect);
            let count = if kind == ShapeKind::Curve4 { 4 } else { 5 };
            let positions = |style: &Style, rect: Rect| {
                let inner = rect.shrink(shape_padding(style, rect.size()));
                style.curve_points.unwrap()[..count]
                    .iter()
                    .map(|p| inner.min + p.to_vec2() * inner.size())
                    .collect::<Vec<_>>()
            };
            let original = positions(&style, rect);
            let destination = egui::pos2(rect.left() - 80.0, rect.top() - 40.0);
            move_curve_point(kind, &mut style, &mut rect, 0, destination);
            let first = positions(&style, rect);
            assert!((first[0] - destination).length() < 0.001);
            for i in 1..count {
                assert!((first[i] - original[i]).length() < 0.001);
            }
            let destination = egui::pos2(rect.right() + 60.0, rect.bottom() + 50.0);
            move_curve_point(kind, &mut style, &mut rect, count - 1, destination);
            let second = positions(&style, rect);
            assert!((second[count - 1] - destination).length() < 0.001);
            for i in 0..count - 1 {
                assert!((second[i] - first[i]).length() < 0.001);
            }
        }
    }

    #[test]
    fn higher_curve_control_points_edit_bends_independently_and_survive_transforms() {
        for kind in [ShapeKind::Curve4, ShapeKind::Curve5] {
            let mut style = Style::default();
            let mut rect = Rect::from_min_size(Pos2::ZERO, Vec2::splat(100.0));
            enable_curve_edit(kind, &mut style, &mut rect);
            let points = style.curve_points.unwrap();
            let count = if kind == ShapeKind::Curve4 { 4 } else { 5 };
            let baseline = render(kind, style, 200, 200);
            style.curve_points.as_mut().unwrap()[1].y = 0.4;
            let edited = style.curve_points.unwrap();
            assert_eq!(edited[0], points[0]);
            assert_eq!(edited[count - 1], points[count - 1]);
            assert_ne!(render(kind, style, 200, 200), baseline);
            let sample = bezier(&edited[..count]);
            assert_eq!(sample[0], edited[0]);
            assert_eq!(*sample.last().unwrap(), edited[count - 1]);
            transform_shape(kind, &mut style, &mut rect, 35.0, true, false);
            assert!(
                style.curve_points.unwrap()[..count]
                    .iter()
                    .all(|p| p.x.is_finite() && p.y.is_finite())
            );
            assert!(render(kind, style, 200, 200).pixels().any(|p| p[3] > 0));
        }
    }

    #[test]
    fn flattened_curve_can_still_be_rotated() {
        let mut style = Style {
            curve_points: Some([
                egui::pos2(0.0, 0.5),
                egui::pos2(0.3, 0.5),
                egui::pos2(0.7, 0.5),
                egui::pos2(1.0, 0.5),
                Pos2::ZERO,
            ]),
            ..Default::default()
        };
        let mut rect = Rect::from_min_size(Pos2::ZERO, Vec2::splat(100.0));
        transform_shape(ShapeKind::Curve4, &mut style, &mut rect, 90.0, false, false);
        assert!(rect.is_finite() && rect.width() > 0.0 && rect.height() > 0.0);
        assert!(
            style
                .curve_points
                .unwrap()
                .iter()
                .all(|p| p.x.is_finite() && p.y.is_finite())
        );
    }

    #[test]
    fn quarter_turns_preserve_center_and_restore_geometry_without_distortion() {
        let original = Rect::from_min_size(egui::pos2(40.0, 50.0), egui::vec2(120.0, 60.0));
        let mut rect = original;
        let mut style = Style::default();
        for turn in 0..4 {
            transform_shape(
                ShapeKind::Capsule,
                &mut style,
                &mut rect,
                90.0,
                false,
                false,
            );
            assert!((rect.center() - original.center()).length() < 0.001);
            let expected = if turn % 2 == 0 {
                egui::vec2(60.0, 120.0)
            } else {
                original.size()
            };
            assert!((rect.size() - expected).length() < 0.001);
        }
        let baseline = render(ShapeKind::Capsule, Style::default(), 120, 60);
        let restored = render(ShapeKind::Capsule, style, 120, 60);
        let error = baseline
            .pixels()
            .zip(restored.pixels())
            .map(|(a, b)| pixel_error(a, b))
            .max()
            .unwrap();
        assert!(error <= 3, "round-trip raster error: {error}");
    }

    #[test]
    fn reflections_mirror_pixels_and_arbitrary_rotation_fits_selector() {
        for horizontal in [true, false] {
            let mut style = Style::default();
            let mut rect = Rect::from_min_size(Pos2::ZERO, egui::vec2(100.0, 80.0));
            let baseline = render(ShapeKind::CheckMark, style, 100, 80);
            transform_shape(
                ShapeKind::CheckMark,
                &mut style,
                &mut rect,
                0.0,
                horizontal,
                !horizontal,
            );
            let reflected = render(ShapeKind::CheckMark, style, 100, 80);
            let geometry_rect = Rect::from_min_size(Pos2::ZERO, egui::vec2(100.0, 80.0));
            let original_paths = outlines(ShapeKind::CheckMark, geometry_rect);
            let mirrored_paths =
                oriented_outlines(ShapeKind::CheckMark, style.transform, geometry_rect);
            for (original, mirrored) in original_paths[0].0.iter().zip(&mirrored_paths[0].0) {
                let expected = egui::pos2(
                    if horizontal {
                        100.0 - original.x
                    } else {
                        original.x
                    },
                    if horizontal {
                        original.y
                    } else {
                        80.0 - original.y
                    },
                );
                assert!((*mirrored - expected).length() < 0.001);
            }
            // Scanline antialiasing can sample a reflected edge differently;
            // compare total visible error while checking geometry exactly above.
            let mut total_error = 0_u32;
            for (x, y, pixel) in reflected.enumerate_pixels() {
                let expected = baseline.get_pixel(
                    if horizontal { 99 - x } else { x },
                    if horizontal { y } else { 79 - y },
                );
                total_error += pixel_error(pixel, expected) as u32;
            }
            assert!(
                total_error < 100 * 80,
                "mean reflection raster error: {}",
                total_error as f32 / 8000.0
            );
        }
        let mut rect = Rect::from_min_size(Pos2::ZERO, egui::vec2(120.0, 60.0));
        let mut style = Style::default();
        transform_shape(ShapeKind::Square, &mut style, &mut rect, 45.0, false, false);
        assert!((rect.width() - 180.0 / 2.0_f32.sqrt()).abs() < 0.001);
        assert!((rect.width() - rect.height()).abs() < 0.001);
        let paths = oriented_outlines(ShapeKind::Square, style.transform, rect);
        let bounds = path_bounds(&paths);
        assert!(
            (bounds.min - rect.min).length() < 0.001 && (bounds.max - rect.max).length() < 0.001
        );
    }

    #[test]
    fn every_shape_renders_visible_pixels_and_full_transparency_clears_it() {
        for kind in ShapeKind::LINES.into_iter().chain(ShapeKind::SHAPES) {
            let visible = render(kind, Style::default(), 100, 80);
            assert!(visible.pixels().any(|p| p[3] > 0), "{}", kind.name());
            let transparent = render(
                kind,
                Style {
                    transparency: 100.0,
                    ..Default::default()
                },
                100,
                80,
            );
            assert!(transparent.pixels().all(|p| p[3] == 0), "{}", kind.name());
        }
    }
    #[test]
    fn marks_have_independent_fills_and_borders_and_can_be_hollow() {
        for kind in [ShapeKind::XMark, ShapeKind::CheckMark] {
            let style = Style {
                border: Color32::RED,
                fill: Color32::BLUE,
                border_width: 2.0,
                transparency: 0.0,
                ..Default::default()
            };
            let filled = render(kind, style, 160, 160);
            let (x, y, _) = filled
                .enumerate_pixels()
                .find(|(_, _, p)| p.0 == [0, 0, 255, 255])
                .expect("mark has a fillable interior");
            assert!(filled.pixels().any(|p| p.0 == [255, 0, 0, 255]));
            let hollow = render(
                kind,
                Style {
                    fill: Color32::TRANSPARENT,
                    ..style
                },
                160,
                160,
            );
            assert_eq!(hollow.get_pixel(x, y)[3], 0);
            assert!(hollow.pixels().any(|p| p.0 == [255, 0, 0, 255]));
        }
    }

    #[test]
    fn translucent_fill_and_border_never_stack_opacity() {
        for kind in ShapeKind::LINES.into_iter().chain(ShapeKind::SHAPES) {
            let image = render(
                kind,
                Style {
                    border_width: 12.0,
                    border: Color32::RED,
                    fill: Color32::BLUE,
                    transparency: 50.0,
                    ..Default::default()
                },
                120,
                100,
            );
            assert!(
                image.pixels().all(|p| p[3] <= 128),
                "{} has overlapping opacity",
                kind.name()
            );
        }
        let hollow = render(
            ShapeKind::Square,
            Style {
                border_width: 12.0,
                border: Color32::TRANSPARENT,
                fill: Color32::BLUE,
                transparency: 0.0,
                ..Default::default()
            },
            100,
            100,
        );
        assert_eq!(
            hollow.get_pixel(8, 50)[3],
            0,
            "fill must not occupy a transparent border's area"
        );
        assert_eq!(hollow.get_pixel(50, 50).0, [0, 0, 255, 255]);
    }

    #[test]
    fn every_shape_fits_its_selector_at_multiple_sizes() {
        for kind in ShapeKind::LINES.into_iter().chain(ShapeKind::SHAPES) {
            for size in [
                egui::vec2(100.0, 100.0),
                egui::vec2(160.0, 70.0),
                egui::vec2(45.0, 120.0),
            ] {
                let rect = Rect::from_min_size(egui::pos2(12.0, 25.0), size);
                let points: Vec<_> = outlines(kind, rect)
                    .into_iter()
                    .flat_map(|(points, _)| points)
                    .collect();
                let bounds = Rect::from_points(&points);
                assert!(
                    (bounds.min - rect.min).length() < 0.001,
                    "{} minimum",
                    kind.name()
                );
                assert!(
                    (bounds.max - rect.max).length() < 0.001,
                    "{} maximum",
                    kind.name()
                );
            }
        }
    }

    #[test]
    fn thought_balloon_has_close_dots_and_a_single_pill_with_attached_bubble() {
        let rect = Rect::from_min_size(egui::Pos2::ZERO, Vec2::splat(100.0));
        let paths = outlines(ShapeKind::ThoughtBalloon, rect);
        assert_eq!(
            paths.len(),
            3,
            "two dots and one joined body, without an internal seam"
        );
        assert!(paths.iter().all(|(_, closed)| *closed));
        let small = Rect::from_points(&paths[0].0);
        let medium = Rect::from_points(&paths[1].0);
        let body = Rect::from_points(&paths[2].0);
        assert!(small.width() < medium.width());
        assert!(body.width() > body.height());
        assert!(small.center().y > medium.center().y);
        assert!(small.top() - medium.bottom() < 4.0);
        assert!(medium.top() - body.bottom() < 4.0);
        let points = &paths[2].0;
        assert!((points[0].y - points[1].y).abs() < 0.001);
        assert!(
            points[1].x - points[0].x > 25.0,
            "pill has a straight top edge"
        );
    }

    #[test]
    fn both_stars_match_selection_bounds_at_different_aspect_ratios() {
        for kind in [ShapeKind::Star5, ShapeKind::Star6] {
            for size in [egui::vec2(100.0, 100.0), egui::vec2(160.0, 70.0)] {
                let rect = Rect::from_min_size(egui::pos2(10.0, 20.0), size);
                let paths = outlines(kind, rect);
                let bounds = Rect::from_points(&paths[0].0);
                assert!((bounds.min - rect.min).length() < 0.001);
                assert!((bounds.max - rect.max).length() < 0.001);
            }
        }
    }

    #[test]
    fn triangle_vertices_match_the_selection_bounds() {
        let rect = Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(120.0, 80.0));
        let triangle = outlines(ShapeKind::Triangle, rect);
        assert_eq!(
            triangle[0].0,
            vec![rect.center_top(), rect.right_bottom(), rect.left_bottom()]
        );
        let image = render(ShapeKind::Triangle, Style::default(), 120, 80);
        let occupied: Vec<_> = image
            .enumerate_pixels()
            .filter(|(_, _, p)| p[3] > 0)
            .map(|(x, y, _)| (x, y))
            .collect();
        assert!(occupied.iter().map(|(x, _)| *x).min().unwrap() <= 3);
        assert!(occupied.iter().map(|(x, _)| *x).max().unwrap() >= 116);
        assert!(occupied.iter().map(|(_, y)| *y).min().unwrap() <= 3);
        assert!(occupied.iter().map(|(_, y)| *y).max().unwrap() >= 76);
    }

    #[test]
    fn fill_border_and_transparency_are_applied_to_raster() {
        let style = Style {
            border_width: 8.0,
            border: Color32::RED,
            fill: Color32::BLUE,
            transparency: 50.0,
            ..Default::default()
        };
        let image = render(ShapeKind::Square, style, 100, 100);
        assert_eq!(image.get_pixel(50, 50).0, [0, 0, 255, 128]);
        assert!(image.pixels().any(|p| p[0] > 200 && p[2] < 20));
        assert_eq!(image.get_pixel(0, 0)[3], 0);
        let borderless = render(
            ShapeKind::Square,
            Style {
                border_width: 0.0,
                ..style
            },
            100,
            100,
        );
        assert!(!borderless.pixels().any(|p| p[0] > 0));
    }
}
