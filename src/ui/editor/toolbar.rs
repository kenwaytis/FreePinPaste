use super::*;
use crate::ui::{
    button,
    icons::{Icon, icon},
};

const PALETTE: [u32; 20] = [
    0x242b36, 0xffffff, 0xf04452, 0xff7b35, 0xffc83d, 0x52c832, 0x20c9d8, 0x2597f4, 0x7568e9,
    0xe75cdb, 0x7d8b9c, 0xe2e8ed, 0xffa9b3, 0xffc3a0, 0xffec98, 0xc2eb9b, 0x9ae6e5, 0xafd6fa,
    0xc8c0f5, 0xf4b8eb,
];
struct Tip(&'static str);
impl Render for Tip {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .px_2()
            .py_1()
            .rounded_md()
            .bg(rgb(0x263747))
            .text_color(white())
            .text_size(px(12.))
            .child(self.0)
    }
}

impl Editor {
    pub(super) fn toolbar(&self, p: P, cx: &mut Context<Self>) -> impl IntoElement {
        let cx: &Context<Self> = cx;
        let tools = [
            (
                Action::Shape,
                Icon::Shape,
                "矩形 / 椭圆 · R",
                matches!(self.tool, Tool::Rectangle | Tool::Ellipse),
                true,
            ),
            (
                Action::Line,
                Icon::Line,
                "直线 / 箭头 · L",
                matches!(self.tool, Tool::Line | Tool::Arrow),
                true,
            ),
            (
                Action::Pen,
                Icon::Pen,
                "自由画笔 · P",
                self.tool == Tool::Pen,
                true,
            ),
            (
                Action::Text,
                Icon::Text,
                "文本 · T",
                self.tool == Tool::Text,
                true,
            ),
            (
                Action::Eraser,
                Icon::Eraser,
                "橡皮擦 · E（只擦除标注）",
                self.tool == Tool::Eraser,
                true,
            ),
            (
                Action::Ocr,
                Icon::Ocr,
                "离线 OCR · O",
                self.panel == Some(Panel::Ocr),
                true,
            ),
            (
                Action::Undo,
                Icon::Undo,
                "撤销 · Ctrl+Z",
                false,
                self.document.can_undo(),
            ),
            (
                Action::Redo,
                Icon::Redo,
                "还原 / 重做 · Ctrl+Shift+Z",
                false,
                self.document.can_redo(),
            ),
            (Action::Close, Icon::Close, "退出截屏 · Esc", false, true),
            (Action::Pin, Icon::Pin, "置顶选区", false, true),
            (Action::Save, Icon::Save, "保存为 PNG · Ctrl+S", false, true),
            (
                Action::Copy,
                Icon::Copy,
                "复制 · Enter / Ctrl+C",
                false,
                true,
            ),
        ];
        let mut main = div()
            .flex()
            .items_center()
            .p(px(4.))
            .gap(px(2.))
            .rounded_md()
            .bg(rgb(0xfffdf7))
            .border_1()
            .border_color(rgb(0xd5dce2))
            .shadow_lg();
        for (index, (action, glyph, label, active, enabled)) in tools.into_iter().enumerate() {
            if [6, 8, 9].contains(&index) {
                main = main.child(div().w(px(1.)).h(px(23.)).mx(px(3.)).bg(rgb(0xd5dce2)));
            }
            main = main.child(
                div()
                    .id(("tool", index))
                    .w(px(38.))
                    .h(px(36.))
                    .flex()
                    .justify_center()
                    .items_center()
                    .rounded_sm()
                    .cursor_pointer()
                    .bg(if active { rgb(0x249fe9) } else { rgb(0xfffdf7) })
                    .hover(move |s| s.bg(if active { rgb(0x158bce) } else { rgb(0xeaf1f5) }))
                    .child(icon(
                        glyph,
                        if !enabled {
                            0xc9cdd0
                        } else if active {
                            0xffffff
                        } else {
                            0x47545d
                        },
                    ))
                    .tooltip(move |_, cx| cx.new(|_| Tip(label)).into())
                    .on_click(cx.listener(move |this, _, window, cx| {
                        if enabled {
                            this.perform(action, window, cx);
                        }
                    })),
            )
        }
        let mut stack = div()
            .occlude()
            .cursor(CursorStyle::Arrow)
            .absolute()
            .left(px(p.x))
            .top(px(p.y))
            .flex()
            .flex_col()
            .gap(px(6.))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
            .child(main);
        if self.tool != Tool::Select {
            let mut sub = div()
                .flex()
                .items_center()
                .gap(px(8.))
                .p(px(6.))
                .h(px(48.))
                .rounded_md()
                .bg(rgb(0xfffdf7))
                .border_1()
                .border_color(rgb(0xd5dce2))
                .shadow_md();
            if self.tool == Tool::Text {
                sub = sub
                    .child(self.option("bold", "B", self.style.bold, cx, |s| {
                        s.style.bold = !s.style.bold
                    }))
                    .child(self.option("italic", "I", self.style.italic, cx, |s| {
                        s.style.italic = !s.style.italic
                    }))
                    .child(
                        button("font", self.style.font_family.clone())
                            .max_w(px(110.))
                            .overflow_hidden()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.style.font_family = if this.style.font_family == "sans-serif" {
                                    "monospace".into()
                                } else if this.style.font_family == "monospace"
                                    && cx.global::<State>().config.font_family != "monospace"
                                {
                                    cx.global::<State>().config.font_family.clone()
                                } else {
                                    "sans-serif".into()
                                };
                                cx.notify();
                            })),
                    )
                    .child(
                        button("font-smaller", "−").on_click(cx.listener(|s, _, _, cx| {
                            s.style.font_size = (s.style.font_size - 2.).max(10.);
                            cx.notify();
                        })),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .child(format!("{}", self.style.font_size)),
                    )
                    .child(
                        button("font-larger", "+").on_click(cx.listener(|s, _, _, cx| {
                            s.style.font_size = (s.style.font_size + 2.).min(120.);
                            cx.notify();
                        })),
                    );
            } else {
                for (index, width) in [1., 3., 6., 10.].into_iter().enumerate() {
                    sub = sub.child(
                        div()
                            .id(("width", index))
                            .w(px(24.))
                            .h(px(30.))
                            .rounded_sm()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .bg(if self.style.width == width {
                                rgb(0xd8efff)
                            } else {
                                rgb(0xfffdf7)
                            })
                            .child(div().size(px(width + 2.)).rounded_full().bg(rgb(0x465767)))
                            .on_click(cx.listener(move |s, _, _, cx| {
                                s.style.width = width;
                                cx.notify();
                            })),
                    );
                }
                match self.tool {
                    Tool::Rectangle | Tool::Ellipse => {
                        sub = sub
                            .child(self.option(
                                "rect",
                                "□",
                                self.tool == Tool::Rectangle,
                                cx,
                                |s| s.tool = Tool::Rectangle,
                            ))
                            .child(self.option(
                                "ellipse",
                                "○",
                                self.tool == Tool::Ellipse,
                                cx,
                                |s| s.tool = Tool::Ellipse,
                            ))
                            .child(self.option("filled", "填充", self.style.filled, cx, |s| {
                                s.style.filled = !s.style.filled
                            }));
                    }
                    Tool::Line | Tool::Arrow => {
                        sub = sub
                            .child(self.option("line", "／", self.tool == Tool::Line, cx, |s| {
                                s.tool = Tool::Line
                            }))
                            .child(self.option(
                                "arrow",
                                "↗",
                                self.tool == Tool::Arrow,
                                cx,
                                |s| s.tool = Tool::Arrow,
                            ));
                    }
                    Tool::Eraser => {
                        sub = sub.child(
                            div()
                                .text_size(px(12.))
                                .child(format!("{} px · 保留原图", self.style.width * 5.)),
                        );
                    }
                    _ => {}
                }
            }
            if self.tool != Tool::Eraser {
                sub = sub
                    .child(div().w(px(1.)).h(px(28.)).bg(rgb(0xd5dce2)))
                    .child(
                        div()
                            .size(px(30.))
                            .rounded_sm()
                            .border_1()
                            .border_color(rgb(0xb8c1c9))
                            .bg(rgb(self.style.color)),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(3.))
                            .children((0..2).map(|row| {
                                div().flex().gap(px(3.)).children((0..10).map(move |col| {
                                    let color = PALETTE[row * 10 + col];
                                    div()
                                        .id(("color", row * 10 + col))
                                        .size(px(14.))
                                        .rounded(px(1.))
                                        .bg(rgb(color))
                                        .border_1()
                                        .border_color(if color == self.style.color {
                                            rgb(0x168be5)
                                        } else {
                                            rgb(0xc4cbd0)
                                        })
                                        .cursor_pointer()
                                        .on_click(cx.listener(move |s, _, _, cx| {
                                            s.style.color = color;
                                            cx.notify();
                                        }))
                                }))
                            })),
                    );
            }
            stack = stack.child(sub);
        }
        stack
    }
    fn option(
        &self,
        id: &'static str,
        label: &'static str,
        active: bool,
        cx: &Context<Self>,
        f: impl Fn(&mut Self) + 'static,
    ) -> impl IntoElement {
        button(id, label)
            .px(px(7.))
            .py(px(6.))
            .bg(if active { rgb(0x249fe9) } else { rgb(0xf2f4f4) })
            .text_color(if active { rgb(0xffffff) } else { rgb(0x465767) })
            .on_click(cx.listener(move |s, _, _, cx| {
                f(s);
                cx.notify();
            }))
    }
    pub(super) fn panel_view(
        &self,
        panel: Panel,
        window: &Window,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let viewport = window.viewport_size();
        let r = self
            .document
            .selection
            .map(|r| self.screen_rect(r))
            .unwrap_or(Rect {
                x: 0.,
                y: 0.,
                w: 0.,
                h: 0.,
            });
        let left = r.x.clamp(8., (f32::from(viewport.width) - 540.).max(8.));
        let top = r.y.clamp(55., (f32::from(viewport.height) - 360.).max(55.));
        div()
            .occlude()
            .cursor(CursorStyle::Arrow)
            .absolute()
            .left(px(left))
            .top(px(top))
            .w(px(520.))
            .p_4()
            .flex()
            .flex_col()
            .gap_3()
            .rounded_lg()
            .bg(rgb(0xfffdf7))
            .border_1()
            .border_color(rgb(0xaccde4))
            .shadow_lg()
            .text_color(rgb(0x263747))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
            .child(
                div()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(if panel == Panel::Text {
                        "文本标注 · Ctrl+Enter 确认"
                    } else {
                        "离线 OCR · CPU"
                    }),
            )
            .child(Input::new(&self.text).h(px(200.)).disabled(self.busy))
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        button(
                            "confirm-panel",
                            if panel == Panel::Text {
                                "添加文本"
                            } else {
                                "复制文字"
                            },
                        )
                        .on_click(cx.listener(
                            move |this, _, window, cx| {
                                if this.busy {
                                    return;
                                }
                                if panel == Panel::Text {
                                    this.commit_text(window, cx);
                                } else {
                                    cx.write_to_clipboard(ClipboardItem::new_string(
                                        this.text.read(cx).value().to_string(),
                                    ));
                                    this.message = "OCR 文字已复制".into();
                                    cx.notify();
                                }
                            },
                        )),
                    )
                    .child(button("cancel-panel", "关闭").on_click(cx.listener(
                        |this, _, window, cx| {
                            if !this.busy {
                                this.panel = None;
                                this.focus.focus(window);
                                cx.notify();
                            }
                        },
                    ))),
            )
    }
}
