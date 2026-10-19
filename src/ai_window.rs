use crate::{QuipIt, toolbar};
use eframe::egui::{self, Color32, Pos2, Rect, Vec2};

#[derive(Clone)]
pub(crate) struct AiWindow {
    pub detached: bool,
    position: Option<Pos2>,
    size: Vec2,
    dragging: bool,
    drag_offset: Vec2,
    dock_screen: Rect,
    dock_local: Rect,
    docked_drop: Option<Pos2>,
    detached_on_frame: Option<u64>,
    floating_reposition: bool,
    native_open: bool,
    native_initial_position: Option<Pos2>,
    native_initial_size: Vec2,
    native_previous_outer: Option<Rect>,
    native_move_active: bool,
    native_header_local: Rect,
}

impl Default for AiWindow {
    fn default() -> Self {
        Self {
            detached: false,
            position: None,
            size: egui::vec2(620.0, 380.0),
            dragging: false,
            drag_offset: Vec2::ZERO,
            dock_screen: Rect::NOTHING,
            dock_local: Rect::NOTHING,
            docked_drop: None,
            detached_on_frame: None,
            floating_reposition: false,
            native_open: false,
            native_initial_position: None,
            native_initial_size: egui::vec2(620.0, 380.0),
            native_previous_outer: None,
            native_move_active: false,
            native_header_local: Rect::NOTHING,
        }
    }
}

impl AiWindow {
    fn observe_native_move(
        &mut self,
        outer: Rect,
        inner: Rect,
        primary_down: Option<bool>,
    ) -> bool {
        if let Some(previous) = self.native_previous_outer {
            let moved = (outer.min - previous.min).length_sq() > 0.25;
            let resized = (outer.size() - previous.size()).length_sq() > 0.25;
            if moved && !resized {
                self.native_move_active = true;
                self.dragging = true;
            }
        }
        self.native_previous_outer = Some(outer);
        if self.native_move_active && primary_down == Some(false) {
            self.native_move_active = false;
            self.dragging = false;
            let title_bar = Rect::from_min_max(outer.min, egui::pos2(outer.right(), inner.top()));
            let internal_header = self.native_header_local.translate(inner.min.to_vec2());
            return self.dock_screen.intersects(title_bar)
                || self.dock_screen.intersects(internal_header);
        }
        false
    }
}

fn screen_origin(ctx: &egui::Context) -> Pos2 {
    ctx.input(|input| input.viewport().inner_rect.map(|rect| rect.min))
        .unwrap_or(Pos2::ZERO)
}

impl QuipIt {
    pub(crate) fn footer(&mut self, ctx: &egui::Context) {
        let available = ctx.available_rect();
        self.ai_window.dock_local = Rect::from_min_max(
            egui::pos2(
                available.left(),
                (available.bottom() - 100.0).max(available.top()),
            ),
            available.max,
        );
        self.ai_window.dock_screen = self
            .ai_window
            .dock_local
            .translate(screen_origin(ctx).to_vec2());
        if !self.show_ai_commands {
            return;
        }
        if self.ai_window.detached {
            let fill = if self.ai_window.dragging {
                Color32::from_rgb(60, 90, 120)
            } else {
                Color32::from_gray(40)
            };
            let target = egui::TopBottomPanel::bottom("ai_dock_landing")
                .exact_height(42.0)
                .frame(egui::Frame::new().fill(fill).inner_margin(4))
                .show(ctx, |ui| {
                    if ui
                        .add_sized(
                            ui.available_size(),
                            egui::Button::new("Drop either window header here to dock · click to dock"),
                        )
                        .on_hover_text(
                            "Drag either the native title bar or the AI header here and release, or click to dock it",
                        )
                        .clicked()
                    {
                        self.dock_ai_window(ctx);
                    }
                });
            self.ai_window.dock_local = target.response.rect;
            self.ai_window.dock_screen =
                target.response.rect.translate(screen_origin(ctx).to_vec2());
            return;
        }
        let panel = egui::TopBottomPanel::bottom("ai_footer")
            .resizable(true)
            .default_height(190.0)
            .min_height(120.0)
            .max_height((available.height() * 0.6).max(120.0))
            .frame(egui::Frame::new().fill(ctx.style().visuals.panel_fill))
            .show_separator_line(false)
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                let background = ui.visuals().panel_fill;
                egui::Frame::new()
                    .fill(Color32::from_rgb(
                        background.r().saturating_sub(6),
                        background.g().saturating_sub(6),
                        background.b().saturating_sub(6),
                    ))
                    .stroke(egui::Stroke::new(1.0_f32, Color32::from_gray(58)))
                    .corner_radius(10)
                    .inner_margin(8)
                    .outer_margin(egui::Margin {
                        left: 4,
                        right: 4,
                        top: 4,
                        bottom: 0,
                    })
                    .show(ui, |ui| {
                        ui.set_min_size(ui.available_size());
                        self.ai_chat_contents(ui, false, false);
                    });
            });
        if let Some(point) = self.ai_window.docked_drop.take()
            && !panel.response.rect.contains(point)
        {
            self.ai_window.detached = true;
            self.ai_window.floating_reposition = true;
            self.ai_window.detached_on_frame = Some(ctx.cumulative_frame_nr());
            self.ai_window.position =
                Some(screen_origin(ctx) + point.to_vec2() - egui::vec2(100.0, 12.0));
            self.ai_window.dragging = false;
            ctx.request_repaint();
        }
    }

    fn ai_chat_contents(&mut self, ui: &mut egui::Ui, detached: bool, native: bool) {
        ui.spacing_mut().item_spacing.y = 6.0;
        let reserved = if detached { 236.0 } else { 172.0 };
        let thinking = self.ai_chat.is_thinking();
        let header = toolbar::section_header_with_reserved_space(
            ui,
            &self.ai_settings.chat_title(),
            reserved + if thinking { 80.0 } else { 0.0 },
        );
        if thinking {
            ui.painter().text(
                header.rect.right_center() - egui::vec2(reserved + 8.0, 0.0),
                egui::Align2::RIGHT_CENTER,
                "Thinking…",
                egui::FontId::proportional(12.0),
                ui.visuals().weak_text_color(),
            );
        }
        if native {
            self.ai_window.native_header_local = header.rect;
        }
        let drag_rect = Rect::from_min_max(
            header.rect.min,
            egui::pos2(
                (header.rect.right() - reserved).max(header.rect.left()),
                header.rect.bottom(),
            ),
        );
        let drag = ui.interact(drag_rect, egui::Id::new("ai_header_drag"), egui::Sense::drag())
            .on_hover_cursor(egui::CursorIcon::Grab)
            .on_hover_text(if detached { "Drag this header to move the window; release over the editor's bottom dock area to dock" } else { "Drag this header out of the panel to detach AI chat" });
        if drag.drag_started() {
            self.ai_window.dragging = true;
            if detached {
                if native {
                    // Let the window manager own the drag. Repeated absolute
                    // position commands race its move events and cause jitter.
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
                } else if let Some(pointer) = ui.input(|input| input.pointer.press_origin()) {
                    let global = screen_origin(ui.ctx()) + pointer.to_vec2();
                    self.ai_window.drag_offset = global - self.ai_window.position.unwrap_or(global);
                }
            }
        }
        if drag.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
            if detached
                && !native
                && let Some(pointer) = ui.input(|input| input.pointer.interact_pos())
            {
                self.ai_window.position =
                    Some(screen_origin(ui.ctx()) + pointer.to_vec2() - self.ai_window.drag_offset);
            }
            ui.ctx().request_repaint_of(egui::ViewportId::ROOT);
        }
        let native_button = if native {
            crate::native_mouse::primary_down()
        } else {
            None
        };
        if (drag.drag_stopped() && (!native || native_button != Some(true)))
            || (native && self.ai_window.dragging && native_button == Some(false))
        {
            self.ai_window.dragging = false;
            if let Some(pointer) = ui.input(|input| input.pointer.interact_pos()) {
                if detached {
                    let global = screen_origin(ui.ctx()) + pointer.to_vec2();
                    if self.ai_window.dock_screen.contains(global) {
                        self.dock_ai_window(ui.ctx());
                    }
                } else {
                    self.ai_window.docked_drop = Some(pointer);
                }
            }
            ui.ctx().request_repaint_of(egui::ViewportId::ROOT);
        }
        if detached {
            let dock_rect = Rect::from_center_size(
                header.rect.right_center() - egui::vec2(200.0, 0.0),
                egui::vec2(60.0, 20.0),
            );
            if ui
                .place(dock_rect, egui::Button::new("Dock"))
                .on_hover_text("Dock AI chat at the bottom of the editor")
                .clicked()
            {
                self.dock_ai_window(ui.ctx());
            }
        }
        let setup_rect = Rect::from_center_size(
            header.rect.right_center() - egui::vec2(98.0, 0.0),
            egui::vec2(132.0, 20.0),
        );
        if ui
            .place(
                setup_rect,
                egui::Button::new(self.ai_settings.button_label()),
            )
            .on_hover_text("Set up or switch AI provider")
            .clicked()
        {
            self.ai_settings.begin();
        }
        let close_rect = Rect::from_center_size(
            header.rect.right_center() - egui::vec2(14.0, 0.0),
            Vec2::splat(20.0),
        );
        if ui
            .place(close_rect, egui::Button::new("×").frame(false))
            .on_hover_text("Close AI chat")
            .clicked()
        {
            self.show_ai_commands = false;
            self.ai_window.dragging = false;
            ui.ctx().request_repaint_of(egui::ViewportId::ROOT);
        }
        if let Some(prompt) = self.ai_chat.ui(ui) {
            self.send_ai(ui.ctx(), prompt);
        }
    }

    pub(crate) fn detached_ai_window(&mut self, ctx: &egui::Context) {
        if !self.show_ai_commands || !self.ai_window.detached {
            self.ai_window.native_open = false;
            self.ai_window.native_previous_outer = None;
            self.ai_window.native_move_active = false;
            return;
        }
        // The docked widgets already rendered on the release frame. Start the
        // floating widgets next frame to keep their IDs on one layer at a time.
        if self.ai_window.detached_on_frame == Some(ctx.cumulative_frame_nr()) {
            ctx.request_repaint();
            return;
        }
        // Wayland does not expose global window coordinates. Keep the floating
        // panel inside the editor there so mouse docking remains available.
        if ctx.embed_viewports() || ctx.input(|input| input.viewport().inner_rect.is_none()) {
            self.floating_ai_window(ctx);
            self.ai_settings.dialog(ctx);
            return;
        }
        if !self.ai_window.native_open {
            self.ai_window.native_initial_position = self.ai_window.position;
            self.ai_window.native_initial_size = self.ai_window.size;
            self.ai_window.native_open = true;
            self.ai_window.native_previous_outer = None;
            self.ai_window.native_move_active = false;
        }
        let id = egui::ViewportId::from_hash_of("quipit_ai_window");
        let mut builder = egui::ViewportBuilder::default()
            .with_title("QuipIt - AI chat")
            .with_decorations(true)
            .with_close_button(false)
            .with_titlebar_buttons_shown(false)
            .with_resizable(true)
            .with_inner_size(self.ai_window.native_initial_size)
            .with_min_inner_size([400.0, 220.0]);
        // Keep the creation hints stable. Echoing observed geometry into this
        // builder sends resize/move commands back while the OS is resizing.
        if let Some(position) = self.ai_window.native_initial_position {
            builder = builder.with_position(position);
        }
        ctx.show_viewport_immediate(id, builder, |ctx, class| {
            if class == egui::ViewportClass::Embedded {
                self.floating_ai_window(ctx);
            } else {
                if ctx.input(|input| input.viewport().close_requested()) {
                    self.show_ai_commands = false;
                    self.ai_window.dragging = false;
                    ctx.request_repaint_of(egui::ViewportId::ROOT);
                    return;
                }
                if let Some(rect) = ctx.input(|input| input.viewport().inner_rect) {
                    self.ai_window.size = rect.size();
                }
                if let Some(rect) = ctx.input(|input| input.viewport().outer_rect) {
                    self.ai_window.position = Some(rect.min);
                }
                if let Some((outer, inner)) = ctx.input(|input| {
                    Some((input.viewport().outer_rect?, input.viewport().inner_rect?))
                }) {
                    if self.ai_window.observe_native_move(
                        outer,
                        inner,
                        crate::native_mouse::primary_down(),
                    ) {
                        self.dock_ai_window(ctx);
                        return;
                    }
                    if self.ai_window.native_move_active {
                        ctx.request_repaint_after(std::time::Duration::from_millis(16));
                        ctx.request_repaint_of(egui::ViewportId::ROOT);
                    }
                }
                egui::CentralPanel::default().show(ctx, |ui| self.ai_chat_contents(ui, true, true));
            }
            self.ai_settings.dialog(ctx);
        });
    }
    fn dock_ai_window(&mut self, ctx: &egui::Context) {
        self.ai_window.detached = false;
        self.ai_window.dragging = false;
        self.ai_window.native_open = false;
        ctx.request_repaint_of(egui::ViewportId::ROOT);
    }

    fn floating_ai_window(&mut self, ctx: &egui::Context) {
        let position = self.ai_window.position.unwrap_or(egui::pos2(80.0, 80.0))
            - screen_origin(ctx).to_vec2();
        let mut window = egui::Window::new("AI chat")
            .id(egui::Id::new("ai_embedded_window"))
            .title_bar(false)
            .default_pos(position)
            .default_size(self.ai_window.size)
            .min_size([400.0, 220.0]);
        // Only our header drag owns placement. Leave idle frames and edge
        // resizing to egui so its left/top resize anchors do not fight us.
        if self.ai_window.dragging || self.ai_window.floating_reposition {
            window = window.current_pos(position);
        }
        self.ai_window.floating_reposition = false;
        if let Some(response) = window.show(ctx, |ui| self.ai_chat_contents(ui, true, false))
            && !self.ai_window.dragging
        {
            self.ai_window.position =
                Some(screen_origin(ctx) + response.response.rect.min.to_vec2());
            self.ai_window.size = response.response.rect.size();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(app: &mut QuipIt, ctx: &egui::Context, events: Vec<egui::Event>) -> egui::FullOutput {
        ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1200.0, 900.0))),
                events,
                ..Default::default()
            },
            |ctx| app.floating_ai_window(ctx),
        )
    }

    fn button(pos: Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        }
    }

    fn window_rect(ctx: &egui::Context) -> Rect {
        ctx.memory(|memory| memory.area_rect(egui::Id::new("ai_embedded_window")))
            .unwrap()
    }

    #[test]
    fn native_title_bar_and_internal_header_both_dock_only_after_mouse_release() {
        for target_top in [600.0, 640.0] {
            let mut window = AiWindow {
                detached: true,
                dock_screen: Rect::from_min_size(
                    egui::pos2(0.0, target_top),
                    egui::vec2(1000.0, 30.0),
                ),
                native_header_local: Rect::from_min_size(Pos2::ZERO, egui::vec2(600.0, 16.0)),
                ..Default::default()
            };
            let outer = Rect::from_min_size(egui::pos2(100.0, 100.0), egui::vec2(620.0, 410.0));
            let inner = Rect::from_min_size(egui::pos2(108.0, 130.0), egui::vec2(604.0, 372.0));
            assert!(!window.observe_native_move(outer, inner, Some(false)));
            let moved_outer = outer.translate(egui::vec2(0.0, 505.0));
            let moved_inner = inner.translate(egui::vec2(0.0, 505.0));
            assert!(!window.observe_native_move(moved_outer, moved_inner, Some(true)));
            assert!(window.native_move_active && window.dragging);
            // No egui header drag or pointer-release event is supplied here.
            assert!(window.observe_native_move(moved_outer, moved_inner, Some(false)));
            assert!(!window.native_move_active && !window.dragging);
        }
    }

    #[test]
    fn native_resize_and_initial_placement_do_not_trigger_docking() {
        let mut window = AiWindow {
            detached: true,
            dock_screen: Rect::from_min_size(egui::pos2(0.0, 600.0), egui::vec2(1000.0, 40.0)),
            native_header_local: Rect::from_min_size(Pos2::ZERO, egui::vec2(600.0, 16.0)),
            ..Default::default()
        };
        let outer = Rect::from_min_size(egui::pos2(100.0, 605.0), egui::vec2(620.0, 410.0));
        let inner = Rect::from_min_size(egui::pos2(108.0, 635.0), egui::vec2(604.0, 372.0));
        assert!(!window.observe_native_move(outer, inner, Some(false)));
        let resized_outer = Rect::from_min_max(outer.min - egui::vec2(20.0, 0.0), outer.max);
        let resized_inner = Rect::from_min_max(inner.min - egui::vec2(20.0, 0.0), inner.max);
        assert!(!window.observe_native_move(resized_outer, resized_inner, Some(true)));
        assert!(!window.observe_native_move(resized_outer, resized_inner, Some(false)));
    }

    #[test]
    fn detached_header_dock_button_reattaches_without_a_native_drop_event() {
        let ctx = egui::Context::default();
        let mut app = QuipIt {
            show_ai_commands: true,
            ..Default::default()
        };
        app.ai_window.detached = true;
        let render = |app: &mut QuipIt, events| {
            ctx.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(620.0, 380.0))),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default()
                        .show(ctx, |ui| app.ai_chat_contents(ui, true, true));
                },
            )
        };
        render(&mut app, vec![]);
        let output = render(&mut app, vec![]);
        let dock = output
            .shapes
            .iter()
            .find_map(|shape| {
                if let egui::Shape::Text(text) = &shape.shape
                    && text.galley.text() == "Dock"
                {
                    return Some(text.pos + text.galley.size() * 0.5);
                }
                None
            })
            .unwrap();
        for pressed in [true, false] {
            render(
                &mut app,
                vec![egui::Event::PointerMoved(dock), button(dock, pressed)],
            );
        }
        assert!(app.show_ai_commands);
        assert!(!app.ai_window.detached);
    }

    #[test]
    fn floating_window_moves_with_the_pointer_and_left_resize_keeps_its_right_edge() {
        let ctx = egui::Context::default();
        let mut app = QuipIt::default();
        app.ai_window.detached = true;
        app.ai_window.position = Some(egui::pos2(150.0, 120.0));
        for _ in 0..4 {
            frame(&mut app, &ctx, vec![]);
        }
        let initial = window_rect(&ctx);
        let title = app.ai_settings.chat_title();
        let output = frame(&mut app, &ctx, vec![]);
        let header = output
            .shapes
            .iter()
            .find_map(|shape| {
                if let egui::Shape::Text(text) = &shape.shape
                    && text.galley.text() == title
                {
                    return Some(text.pos + text.galley.size() * 0.5);
                }
                None
            })
            .unwrap();
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(header), button(header, true)],
        );
        for dx in [10.0, 20.0, 30.0, 40.0, 50.0, 60.0] {
            frame(
                &mut app,
                &ctx,
                vec![egui::Event::PointerMoved(header + egui::vec2(dx, 0.0))],
            );
        }
        frame(
            &mut app,
            &ctx,
            vec![button(header + egui::vec2(60.0, 0.0), false)],
        );
        let moved = window_rect(&ctx);
        assert!(
            (moved.left() - initial.left() - 60.0).abs() <= 1.0,
            "{initial:?} -> {moved:?}"
        );
        for _ in 0..4 {
            frame(&mut app, &ctx, vec![]);
            assert_eq!(window_rect(&ctx), moved);
        }
        let edge = moved.left_center();
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(edge), button(edge, true)],
        );
        let mut previous_left = moved.left();
        for dx in [10.0, 20.0, 30.0, 40.0, 50.0, 60.0] {
            frame(
                &mut app,
                &ctx,
                vec![egui::Event::PointerMoved(edge + egui::vec2(dx, 0.0))],
            );
            let resized = window_rect(&ctx);
            assert!(
                resized.left() >= previous_left - 1.0,
                "Resize reversed direction: {resized:?}"
            );
            assert!(
                (resized.right() - moved.right()).abs() <= 1.0,
                "Right edge drifted: {resized:?}"
            );
            previous_left = resized.left();
        }
        frame(
            &mut app,
            &ctx,
            vec![button(edge + egui::vec2(60.0, 0.0), false)],
        );
        let resized = window_rect(&ctx);
        assert!((resized.left() - moved.left() - 60.0).abs() <= 1.0);
        for _ in 0..4 {
            frame(&mut app, &ctx, vec![]);
            assert_eq!(window_rect(&ctx), resized);
        }
    }

    #[test]
    fn native_header_starts_one_os_drag_without_absolute_position_commands() {
        let ctx = egui::Context::default();
        let mut app = QuipIt::default();
        let render = |app: &mut QuipIt, events| {
            ctx.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(620.0, 380.0))),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default()
                        .show(ctx, |ui| app.ai_chat_contents(ui, true, true));
                },
            )
        };
        render(&mut app, vec![]);
        let title = app.ai_settings.chat_title();
        let output = render(&mut app, vec![]);
        let header = output
            .shapes
            .iter()
            .find_map(|shape| {
                if let egui::Shape::Text(text) = &shape.shape
                    && text.galley.text() == title
                {
                    return Some(text.pos + text.galley.size() * 0.5);
                }
                None
            })
            .unwrap();
        let pressed = render(
            &mut app,
            vec![egui::Event::PointerMoved(header), button(header, true)],
        );
        let mut starts = pressed
            .viewport_output
            .values()
            .flat_map(|viewport| &viewport.commands)
            .filter(|command| matches!(command, egui::ViewportCommand::StartDrag))
            .count();
        for dx in [10.0, 20.0, 30.0] {
            let output = render(
                &mut app,
                vec![egui::Event::PointerMoved(header + egui::vec2(dx, 0.0))],
            );
            for viewport in output.viewport_output.values() {
                for command in &viewport.commands {
                    starts += usize::from(matches!(command, egui::ViewportCommand::StartDrag));
                    assert!(!matches!(command, egui::ViewportCommand::OuterPosition(_)));
                }
            }
        }
        assert_eq!(starts, 1);
    }
}
