// SPDX-License-Identifier: MIT

//! Standalone window mode - can be launched independently of the panel

#[allow(unused_imports)]
use crate::fl;
#[allow(dead_code)]
use crate::process::{ProcessError, ProcessInfo, ProcessManager, SortBy};
use cosmic::iced::{Alignment, Length};
use cosmic::prelude::*;
use cosmic::widget;
use futures_util::SinkExt;
use std::time::Duration;

#[allow(dead_code)]
pub struct StandaloneApp {
    core: cosmic::Core,
    process_manager: ProcessManager,
    processes: Vec<ProcessInfo>,
    show_all: bool,
    sort_by: SortBy,
    search_query: String,
    selected_process: Option<ProcessInfo>,
    confirmation_mode: Option<ConfirmationMode>,
    toast: Option<Toast>,
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum ConfirmationMode {
    Kill,
    ForceKill,
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub struct Toast {
    message: String,
    is_error: bool,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum Message {
    RefreshProcesses,
    KillProcess(u32),
    ForceKillProcess(u32),
    ToggleShowAll(bool),
    SortBy(SortBy),
    UpdateSearch(String),
    SelectProcess(Option<u32>),
    ConfirmKill,
    ConfirmForceKill,
    CancelConfirmation,
    ShowToast(String, bool),
    ClearToast,
    Close,
}

impl cosmic::Application for StandaloneApp {
    type Executor = cosmic::executor::Default;
    type Flags = ();
    type Message = Message;
    const APP_ID: &'static str = "io.github.marcossl10.CosmicProcessKiller.Standalone";

    fn core(&self) -> &cosmic::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut cosmic::Core {
        &mut self.core
    }

    fn init(
        core: cosmic::Core,
        _flags: Self::Flags,
    ) -> (Self, Task<cosmic::Action<Self::Message>>) {
        let mut app = StandaloneApp {
            core,
            process_manager: ProcessManager::new(),
            processes: Vec::new(),
            show_all: false,
            sort_by: SortBy::Cpu,
            search_query: String::new(),
            selected_process: None,
            confirmation_mode: None,
            toast: None,
        };

        app.refresh_processes();

        (app, Task::none())
    }

    fn view(&self) -> Element<'_, Self::Message> {
        let mut content = widget::column().spacing(12).padding(20);

        // Header
        let header = widget::row()
            .spacing(12)
            .align_y(Alignment::Center)
            .push(widget::text(fl!("app-title")).size(24))
            .push(widget::horizontal_space())
            .push(widget::tooltip(
                    widget::button::icon(widget::icon::from_name("view-refresh-symbolic"))
                        .on_press(Message::RefreshProcesses)
                        .padding(8),
                    widget::text(fl!("refresh-tooltip")),
                    widget::tooltip::Position::Bottom,
                )
            )
            .push(
                widget::button::icon(widget::icon::from_name("window-close-symbolic"))
                    .on_press(Message::Close)
                    .padding(8),
            );

        content = content.push(header);

        // Search
        let search = widget::text_input(fl!("search-placeholder"), &self.search_query)
            .on_input(Message::UpdateSearch)
            .width(Length::Fill);

        content = content.push(search);

        // Filter
        let filter_row = widget::row()
            .spacing(8)
            .align_y(Alignment::Center)
            .push(widget::text(fl!("show-all")))
            .push(widget::toggler(self.show_all).on_toggle(Message::ToggleShowAll));

        content = content.push(filter_row);

        // Column Headers
        let header_row = widget::row()
            .spacing(12)
            .padding([0, 5])
            .push(
                widget::button::custom(
                    widget::text(fl!("header-name"))
                        .width(Length::Fill)
                        .align_x(cosmic::iced::alignment::Horizontal::Center),
                )
                    .on_press(Message::SortBy(SortBy::Name))
                    .padding(0)
                    .class(cosmic::theme::Button::Text)
                    .width(Length::Fixed(180.0))
            )
            .push(
                widget::button::custom(
                    widget::text(fl!("header-pid"))
                        .width(Length::Fill)
                        .align_x(cosmic::iced::alignment::Horizontal::Center),
                )
                    .on_press(Message::SortBy(SortBy::Pid))
                    .padding(0)
                    .class(cosmic::theme::Button::Text)
                    .width(Length::Fixed(90.0))
            )
            .push(
                widget::button::custom(
                    widget::text(fl!("header-cpu"))
                        .width(Length::Fill)
                        .align_x(cosmic::iced::alignment::Horizontal::Center),
                )
                    .on_press(Message::SortBy(SortBy::Cpu))
                    .padding(0)
                    .class(cosmic::theme::Button::Text)
                    .width(Length::Fixed(80.0))
            )
            .push(
                widget::button::custom(
                    widget::text(fl!("header-mem"))
                        .width(Length::Fill)
                        .align_x(cosmic::iced::alignment::Horizontal::Center),
                )
                    .on_press(Message::SortBy(SortBy::Memory))
                    .padding(0)
                    .class(cosmic::theme::Button::Text)
                    .width(Length::Fixed(90.0))
            )
            .push(widget::horizontal_space())
            .push(widget::text(fl!("header-actions")).size(14).width(Length::Fixed(100.0))); // Placeholder for alignment

        content = content.push(header_row);

        // Confirmation dialog overlay
        if let (Some(process), Some(mode)) = (&self.selected_process, &self.confirmation_mode) {
            let dialog = widget::column()
                .spacing(12)
                .padding(16)
                .push(
                    widget::text(
                        if matches!(mode, ConfirmationMode::ForceKill) {
                            fl!("confirm-force-kill-message")
                        } else {
                            fl!("confirm-kill-message")
                        }
                    ).size(14)
                )
                .push(
                    widget::text(format!("{} (PID: {})", process.name, process.pid))
                        .size(12)
                )
                .push(
                    widget::row()
                        .spacing(8)
                        .push(
                            widget::button::destructive(fl!("confirm"))
                                .on_press(if matches!(mode, ConfirmationMode::ForceKill) {
                                    Message::ConfirmForceKill
                                } else {
                                    Message::ConfirmKill
                                })
                        )
                        .push(
                            widget::button::text(fl!("cancel"))
                                .on_press(Message::CancelConfirmation)
                        )
                );

            content = content.push(dialog);
        }

        // Process list
        let mut process_list = widget::list_column().spacing(4);

        let filtered_processes = self.get_filtered_processes();

        if filtered_processes.is_empty() {
            process_list = process_list.add(
                widget::container(widget::text(fl!("no-processes")))
                    .padding(20)
                    .center_x(Length::Fill),
            );
        } else {
            for process in filtered_processes.iter() {
                let process_row = self.create_process_row(process);
                process_list = process_list.add(process_row);
            }
        }

        let scrollable = widget::scrollable(process_list)
            .height(Length::Fixed(400.0))
            .width(Length::Fill);

        content = content.push(scrollable);

        // Footer
        let count = filtered_processes.len();
        let info = widget::text(fl!("process-count", count = count)).size(12);
        content = content.push(info);

        // Toast notification
        if let Some(ref toast) = self.toast {
            let toast_text = widget::text(&toast.message)
                .size(14);
            
            let toast_content = widget::row()
                .spacing(8)
                .align_y(Alignment::Center)
                .push(toast_text)
                .padding(12);
            
            content = content.push(toast_content);
        }

        widget::container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn subscription(&self) -> cosmic::iced::Subscription<Self::Message> {
        struct RefreshSubscription;

        cosmic::iced::Subscription::run_with_id(
            std::any::TypeId::of::<RefreshSubscription>(),
            cosmic::iced::stream::channel(4, move |mut channel| async move {
                loop {
                    tokio::time::sleep(Duration::from_secs(2)).await;
                    _ = channel.send(Message::RefreshProcesses).await;
                }
            }),
        )
    }

    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        match message {
            Message::RefreshProcesses => {
                self.refresh_processes();
            }
            Message::KillProcess(pid) => {
                self.handle_kill_process(pid, false);
            }
            Message::ForceKillProcess(pid) => {
                self.handle_kill_process(pid, true);
            }
            Message::ConfirmKill => {
                if let Some(process) = self.selected_process.clone() {
                    self.execute_kill(&process, false);
                }
                self.confirmation_mode = None;
                self.selected_process = None;
            }
            Message::ConfirmForceKill => {
                if let Some(process) = self.selected_process.clone() {
                    self.execute_kill(&process, true);
                }
                self.confirmation_mode = None;
                self.selected_process = None;
            }
            Message::CancelConfirmation => {
                self.confirmation_mode = None;
                self.selected_process = None;
            }
            Message::ToggleShowAll(show_all) => {
                self.show_all = show_all;
                self.refresh_processes();
            }
            Message::SortBy(sort_by) => {
                self.sort_by = sort_by;
                self.refresh_processes();
            }
            Message::UpdateSearch(query) => {
                self.search_query = query;
            }
            Message::SelectProcess(pid) => {
                if let Some(pid) = pid {
                    self.selected_process = self.processes.iter()
                        .find(|p| p.pid == pid)
                        .cloned();
                } else {
                    self.selected_process = None;
                }
            }
            Message::ShowToast(message, is_error) => {
                self.toast = Some(Toast { message, is_error });
            }
            Message::ClearToast => {
                self.toast = None;
            }
            Message::Close => {
                return cosmic::iced::exit();
            }
        }
        Task::none()
    }
}

impl StandaloneApp {
    #[allow(dead_code)]
    fn refresh_processes(&mut self) {
        let mut processes = self.process_manager.get_processes(self.sort_by);
        if !self.show_all {
            processes.truncate(10);
        }
        self.processes = processes;
    }

    #[allow(dead_code)]
    fn get_filtered_processes(&self) -> Vec<&ProcessInfo> {
        if self.search_query.is_empty() {
            self.processes.iter().collect()
        } else {
            let query = self.search_query.to_lowercase();
            self.processes
                .iter()
                .filter(|p| {
                    p.name.to_lowercase().contains(&query)
                        || p.pid.to_string().contains(&query)
                })
                .collect()
        }
    }

    #[allow(dead_code)]
    fn handle_kill_process(&mut self, pid: u32, force: bool) {
        // Find the process
        let process = match self.processes.iter().find(|p| p.pid == pid) {
            Some(p) => p.clone(),
            None => {
                self.toast = Some(Toast {
                    message: fl!("error-process-not-found"),
                    is_error: true,
                });
                return;
            }
        };

        // Check permissions before showing confirmation
        match self.process_manager.can_kill_process(&process) {
            Err(ProcessError::PermissionDenied) => {
                self.toast = Some(Toast {
                    message: fl!("notification-permission-denied"),
                    is_error: true,
                });
                return;
            }
            Err(ProcessError::Protected(name)) => {
                self.toast = Some(Toast {
                    message: fl!("notification-protected", name = name),
                    is_error: true,
                });
                return;
            }
            Err(e) => {
                self.toast = Some(Toast {
                    message: format!("{}: {:?}", fl!("error-unknown-error"), e),
                    is_error: true,
                });
                return;
            }
            Ok(()) => {}
        }

        // Show confirmation dialog
        self.selected_process = Some(process);
        self.confirmation_mode = Some(if force {
            ConfirmationMode::ForceKill
        } else {
            ConfirmationMode::Kill
        });
    }

    #[allow(dead_code)]
    fn execute_kill(&mut self, process: &ProcessInfo, force: bool) {
        let result = if force {
            self.process_manager.force_kill_process(process.pid)
        } else {
            self.process_manager.kill_process(process.pid)
        };

        match result {
            Ok(()) => {
                self.toast = Some(Toast {
                    message: if force {
                        fl!("notification-force-kill-success", name = process.name.clone())
                    } else {
                        fl!("notification-kill-success", name = process.name.clone())
                    },
                    is_error: false,
                });
                self.refresh_processes();
            }
            Err(e) => {
                let error_msg = match e {
                    ProcessError::SignalFailed(msg) => {
                        if force {
                            fl!("error-sigkill-failed", error = msg)
                        } else {
                            fl!("error-sigterm-failed", error = msg)
                        }
                    }
                    ProcessError::PermissionDenied => fl!("notification-permission-denied"),
                    ProcessError::NotFound => fl!("error-process-not-found"),
                    ProcessError::Protected(name) => {
                        fl!("notification-protected", name = name)
                    }
                    ProcessError::Unknown(msg) => {
                        fl!("error-unknown-error", error = msg)
                    }
                };
                
                self.toast = Some(Toast {
                    message: fl!("notification-kill-failed", error = error_msg),
                    is_error: true,
                });
                self.refresh_processes();
            }
        }
    }

    #[allow(dead_code)]
    fn create_process_row<'a>(&self, process: &'a ProcessInfo) -> Element<'a, Message> {
        let is_selected = self.selected_process.as_ref().map(|p| p.pid) == Some(process.pid);

        // Truncate name if too long
        let display_name = if process.name.len() > 25 {
            format!("{}...", &process.name[..22])
        } else {
            process.name.clone()
        };

        let name_text = widget::text(display_name.clone())
            .size(14)
            .width(Length::Fixed(180.0));

        let pid_text = widget::text(format!("PID: {}", process.pid))
            .size(12)
            .width(Length::Fixed(90.0))
            .align_x(cosmic::iced::alignment::Horizontal::Center);

        let cpu_text = widget::text(format!("{:.1}%", process.cpu_usage))
            .size(12)
            .width(Length::Fixed(80.0))
            .align_x(cosmic::iced::alignment::Horizontal::Center);

        let memory_text = widget::text(format!("{} MB", process.memory / 1024 / 1024))
            .size(12)
            .width(Length::Fixed(90.0))
            .align_x(cosmic::iced::alignment::Horizontal::Center);

        // Check if process can be killed
        let can_kill = self.process_manager.can_kill_process(process).is_ok();

        // Action buttons
        let kill_button = widget::tooltip(
            widget::button::custom(widget::icon::from_name("process-stop-symbolic"))
                .on_press(Message::KillProcess(process.pid))
                .padding(4)
                .class(cosmic::theme::Button::Text),
            widget::text(fl!("kill-tooltip")),
            widget::tooltip::Position::Top,
        );

        let force_kill_button = widget::tooltip(
            widget::button::custom(widget::icon::from_name("edit-delete-symbolic"))
                .on_press(Message::ForceKillProcess(process.pid))
                .padding(4)
                .class(cosmic::theme::Button::Text),
            widget::text(fl!("force-kill-tooltip")),
            widget::tooltip::Position::Top,
        );

        let buttons: cosmic::widget::Row<'_, Message> = if can_kill {
            widget::row()
                .spacing(6)
                .push(kill_button)
                .push(force_kill_button)
        } else {
            widget::row()
                .spacing(6)
                .push(
                    widget::button::icon(widget::icon::from_name("lock-symbolic"))
                        .padding(4)
                )
        };

        let info_row = widget::row()
            .spacing(12)
            .align_y(Alignment::Center)
            .push(name_text)
            .push(pid_text)
            .push(cpu_text)
            .push(memory_text)
            .push(widget::horizontal_space());

        let info_button = widget::button::custom(info_row)
            .on_press(Message::SelectProcess(Some(process.pid)))
            .padding([10, 5])
            .width(Length::Fill)
            .class(if is_selected {
                cosmic::theme::Button::Suggested
            } else {
                cosmic::theme::Button::Text
            });

        widget::row()
            .spacing(2)
            .align_y(Alignment::Center)
            .push(info_button)
            .push(buttons)
            .into()
    }
}
