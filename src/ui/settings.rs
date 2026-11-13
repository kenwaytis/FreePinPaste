use super::{State, button};
use crate::{
    config::{self, PinCloseAction},
    service::Command,
};
use gpui::{prelude::*, *};
use gpui_component::input::{Input, InputState};

pub struct Settings {
    capture: Entity<InputState>,
    pin: Entity<InputState>,
    pin_close_action: PinCloseAction,
    languages: Entity<InputState>,
    font: Entity<InputState>,
    save_dir: Entity<InputState>,
    autostart: bool,
    message: String,
    models: String,
}
impl Settings {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let c = cx.global::<State>().config.clone();
        let capture = cx.new(|cx| InputState::new(window, cx).default_value(c.capture_shortcut));
        let pin = cx.new(|cx| InputState::new(window, cx).default_value(c.pin_shortcut));
        let languages = cx.new(|cx| InputState::new(window, cx).default_value(c.ocr_languages));
        let font = cx.new(|cx| InputState::new(window, cx).default_value(c.font_family));
        let save_dir = cx.new(|cx| {
            InputState::new(window, cx).default_value(c.save_directory.display().to_string())
        });
        let models = crate::ocr::Ocr::discover()
            .and_then(|o| o.languages())
            .map(|l| l.join(" · "))
            .unwrap_or_else(|e| e.to_string());
        Self {
            capture,
            pin,
            pin_close_action: c.pin_close_action,
            languages,
            font,
            save_dir,
            autostart: config::autostart_path().exists(),
            message: String::new(),
            models,
        }
    }
    fn save(&mut self, cx: &mut Context<Self>) {
        let config = config::Config {
            capture_shortcut: self.capture.read(cx).value().trim().to_string(),
            pin_shortcut: self.pin.read(cx).value().trim().to_string(),
            pin_close_action: self.pin_close_action,
            ocr_languages: self.languages.read(cx).value().trim().to_string(),
            font_family: self.font.read(cx).value().to_string(),
            save_directory: self.save_dir.read(cx).value().as_ref().into(),
        };
        match config
            .save()
            .and_then(|_| config::set_autostart(self.autostart))
        {
            Ok(()) => {
                cx.global_mut::<State>().config = config;
                self.message =
                    "已保存。默认快捷键供首次绑定使用；已有绑定请点击「系统改键」。".into();
            }
            Err(e) => self.message = format!("保存失败：{e:#}"),
        }
        cx.notify();
    }
}
fn field(label: &'static str, input: &Entity<InputState>) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap_4()
        .child(div().w(px(110.)).text_size(px(13.)).child(label))
        .child(div().flex_1().child(Input::new(input)))
}
impl Render for Settings {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let status = cx.global::<State>().status.clone();
        let authorized = cx.global::<State>().capture_authorized;
        div()
            .id("settings")
            .size_full()
            .overflow_y_scroll()
            .bg(rgb(0xf6f8fa))
            .text_color(rgb(0x243444))
            .p_6()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                div()
                    .flex()
                    .justify_between()
                    .items_center()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .text_size(px(28.))
                                    .font_weight(FontWeight::BOLD)
                                    .child("FreePinPaste"),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(rgb(0x63778a))
                                    .child("截图 · 标注 · 贴图 · 离线文字识别"),
                            ),
                    )
                    .child(
                        button(
                            "capture-now",
                            if authorized {
                                "立即截图"
                            } else {
                                "授权截图"
                            },
                        )
                        .bg(rgb(0x168be5))
                        .text_color(white())
                        .on_click(move |_, _, cx| {
                            super::send(
                                if authorized {
                                    Command::Capture
                                } else {
                                    Command::AuthorizeCapture
                                },
                                cx,
                            )
                        }),
                    ),
            )
            .child(
                div()
                    .p_3()
                    .rounded_lg()
                    .bg(rgb(0xe8f3fb))
                    .text_size(px(13.))
                    .child(status),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .p_4()
                    .bg(white())
                    .rounded_lg()
                    .border_1()
                    .border_color(rgb(0xe3e9ef))
                    .child(div().font_weight(FontWeight::SEMIBOLD).child("全局快捷键"))
                    .child(field("默认截图", &self.capture))
                    .child(field("默认置顶", &self.pin))
                    .child(div().text_size(px(12.)).text_color(rgb(0x748494)).child(
                        "F3：编辑中置顶选区；其他时候置顶剪贴板图片，没有图片时使用最近截图。",
                    ))
                    .child(button("configure", "系统改键…").on_click(|_, _, cx| {
                        let _ = cx
                            .global::<State>()
                            .portal
                            .try_send(crate::portal::Request::ConfigureShortcuts);
                    })),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .p_4()
                    .bg(white())
                    .rounded_lg()
                    .border_1()
                    .border_color(rgb(0xe3e9ef))
                    .child(div().font_weight(FontWeight::SEMIBOLD).child("贴图"))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(div().w(px(110.)).text_size(px(13.)).child("关闭方式"))
                            .children([
                                ("pin-close-double-click", "双击左键关闭（默认）", PinCloseAction::DoubleClick),
                                ("pin-close-right-click", "右键关闭", PinCloseAction::RightClick),
                            ].into_iter().map(|(id, label, action)| {
                                button(id, label)
                                    .when(self.pin_close_action == action, |button| {
                                        button.bg(rgb(0x168be5)).text_color(white())
                                    })
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.pin_close_action = action;
                                        cx.notify();
                                    }))
                            })),
                    )
                    .child(div().text_size(px(12.)).text_color(rgb(0x748494)).child(
                        if self.pin_close_action == PinCloseAction::DoubleClick {
                            "双击左键关闭贴图，右键不关闭；仍可拖动移动、滚轮缩放、Esc 关闭。保存后立即生效。"
                        } else {
                            "右键关闭贴图，双击左键恢复原始尺寸；仍可拖动移动、滚轮缩放、Esc 关闭。保存后立即生效。"
                        },
                    )),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .p_4()
                    .bg(white())
                    .rounded_lg()
                    .border_1()
                    .border_color(rgb(0xe3e9ef))
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("离线 OCR · CPU"),
                    )
                    .child(field("识别语言", &self.languages))
                    .child(div().text_size(px(12.)).text_color(rgb(0x748494)).child(
                        "模型名用 + 分隔。中文推荐 chi_sim+eng；日文 jpn+eng。只加载所选语言。",
                    ))
                    .child(
                        div()
                            .text_size(px(12.))
                            .child(format!("随包模型：{}", self.models)),
                    ),
            )
            .child(field("标注字体", &self.font))
            .child(field("保存目录", &self.save_dir))
            .child(
                button(
                    "autostart",
                    if self.autostart {
                        "☑ 登录桌面后自动启动"
                    } else {
                        "☐ 登录桌面后自动启动"
                    },
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.autostart = !this.autostart;
                    cx.notify();
                })),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        button("save-settings", "保存设置")
                            .on_click(cx.listener(|this, _, _, cx| this.save(cx))),
                    )
                    .child(button("background", "隐藏到后台").on_click(|_, w, _| w.remove_window()))
                    .child(button("exit", "退出程序").on_click(|_, _, cx| cx.quit())),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(rgb(0x39719c))
                    .child(self.message.clone()),
            )
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(rgb(0x8b98a5))
                    .child(format!(
                        "GNOME Wayland / Screenshot Portal / GPUI XWayland\n{}",
                        config::config_path().display()
                    )),
            )
    }
}
