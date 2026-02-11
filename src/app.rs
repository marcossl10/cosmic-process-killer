// SPDX-License-Identifier: MIT

use crate::config::Config;
use crate::fl;
use crate::process::{ProcessError, ProcessInfo, ProcessManager, SortBy};
use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::iced::{window::Id, Alignment, Length, Limits, Subscription};
use cosmic::iced_winit::commands::popup::{destroy_popup, get_popup};
use cosmic::prelude::*;
use cosmic::widget;
use futures_util::SinkExt;
use std::time::Duration;

/// The application model stores app-specific state used to describe its interface and
/// drive its logic.
pub struct AppModel {
    /// Application state which is managed by the COSMIC runtime.
    core: cosmic::Core,
    /// The popup id.
    popup: Option<Id>,
    /// Configuration data that persists between application runs.
    config: Config,
    /// Process manager
    process_manager: ProcessManager,
    /// List of processes
    processes: Vec<ProcessInfo>,
    /// Show all processes or only high CPU
    show_all: bool,
    /// Sort order
    sort_by: SortBy,
    /// Search filter
    search_query: String,
    /// Selected process for confirmation
    selected_process: Option<ProcessInfo>,
    /// Confirmation dialog state
    confirmation_mode: Option<ConfirmationMode>,
    /// Toast notification state
    toast: Option<Toast>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConfirmationMode {
    Kill,
    ForceKill,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Toast {
    message: String,
    is_error: bool,
}

impl Default for AppModel {
    fn default() -> Self {
        Self {
            core: cosmic::Core::default(),
            popup: None,
            config: Config::default(),
            process_manager: ProcessManager::new(),
            processes: Vec::new(),
            show_all: false,
            sort_by: SortBy::Cpu,
            search_query: String::new(),
            selected_process: None,
            confirmation_mode: None,
            toast: None,
        }
    }
}

/// Messages emitted by the application and its widgets.
#[derive(Debug, Clone)]
pub enum Message {
    TogglePopup,
    PopupClosed(Id),
    SubscriptionChannel,
    UpdateConfig(Config),
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
}

/// Create a COSMIC application from the app model
impl cosmic::Application for AppModel {
    /// The async executor that will be used to run your application's commands.
    type Executor = cosmic::executor::Default;

    /// Data that your application receives to its init method.
    type Flags = ();

    /// Messages which the application and its widgets will emit.
    type Message = Message;

    /// Unique identifier in RDNN (reverse domain name notation) format.
    const APP_ID: &'static str = "com.system.CosmicProcessKiller";

    fn core(&self) -> &cosmic::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut cosmic::Core {
        &mut self.core
    }

    /// Initializes the application with any given flags and startup commands.
    fn init(
        core: cosmic::Core,
        _flags: Self::Flags,
    ) -> (Self, Task<cosmic::Action<Self::Message>>) {
        // Construct the app model with the runtime's core.
        let mut app = AppModel {
            core,
            config: cosmic_config::Config::new(Self::APP_ID, Config::VERSION)
                .map(|context| match Config::get_entry(&context) {
                    Ok(config) => config,
                    Err((_errors, config)) => config,
                })
                .unwrap_or_default(),
            ..Default::default()
        };

        // Load initial processes
        app.refresh_processes();

        (app, Task::none())
    }

    fn on_close_requested(&self, id: Id) -> Option<Message> {
        Some(Message::PopupClosed(id))
    }

    /// Describes the interface based on the current state of the application model.
    fn view(&self) -> Element<'_, Self::Message> {
        self.core
            .applet
            .icon_button("process-stop-symbolic")
            .on_press(Message::TogglePopup)
            .into()
    }

    /// The applet's popup window will be drawn using this view method.
    fn view_window(&self, _id: Id) -> Element<'_, Self::Message> {
        let mut content = widget::column().spacing(4).padding(8);

        // Header with title and refresh button
        let header = widget::row()
            .spacing(4)
            .align_y(Alignment::Center)
            .push(widget::text(fl!("app-title")).size(14))
            .push(widget::horizontal_space())
            .push(widget::tooltip(
                    widget::button::icon(widget::icon::from_name("view-refresh-symbolic"))
                        .on_press(Message::RefreshProcesses)
                        .padding(4),
                    widget::text(fl!("refresh-tooltip")),
                    widget::tooltip::Position::Bottom,
                )
            );

        content = content.push(header);

        // Search bar
        let search = widget::text_input(fl!("search-placeholder"), &self.search_query)
            .on_input(Message::UpdateSearch)
            .width(Length::Fill);

        content = content.push(search);

        // Filter controls
        let filter_row = widget::row()
            .spacing(4)
            .align_y(Alignment::Center)
            .push(widget::text(fl!("show-all")).size(12))
            .push(widget::toggler(self.show_all).on_toggle(Message::ToggleShowAll));

        content = content.push(filter_row);

        // Column Headers
        let header_row = widget::row()
            .spacing(4)
            .padding([0, 0])
            .push(
                widget::button::custom(
                    widget::text(fl!("header-name"))
                        .width(Length::Fill)
                        .align_x(cosmic::iced::alignment::Horizontal::Center),
                )
                    .on_press(Message::SortBy(SortBy::Name))
                    .padding(0)
                    .class(cosmic::theme::Button::Text)
                    .width(Length::Fixed(85.0))
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
                    .width(Length::Fixed(60.0))
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
                    .width(Length::Fixed(60.0))
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
                    .width(Length::Fixed(70.0))
            );
        content = content.push(header_row);

        // Confirmation dialog overlay
        if let (Some(process), Some(mode)) = (&self.selected_process, &self.confirmation_mode) {
            let dialog = widget::column()
                .spacing(8)
                .padding(12)
                .push(
                    widget::text(
                        if matches!(mode, ConfirmationMode::ForceKill) {
                            fl!("confirm-force-kill-message")
                        } else {
                            fl!("confirm-kill-message")
                        }
                    ).size(12)
                )
                .push(
                    widget::text(format!("{} (PID: {})", process.name, process.pid))
                        .size(11)
                )
                .push(
                    widget::row()
                        .spacing(4)
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

            content = content.push(
                widget::container(dialog)
                    .class(cosmic::theme::Container::Card)
                    .padding(4),
            );
        }

        // Process list with actions
        let mut process_list = widget::list_column().spacing(2);

        let filtered_processes = self.get_filtered_processes();

        if filtered_processes.is_empty() {
            process_list = process_list.add(
                widget::container(widget::text(fl!("no-processes")))
                    .padding(10)
                    .center_x(Length::Fill),
            );
        } else {
            for process in filtered_processes.iter() {
                let process_row = self.create_process_row(process);
                process_list = process_list.add(process_row);
            }
        }

        let scrollable = widget::scrollable(process_list)
            .height(Length::Fixed(300.0))
            .width(Length::Fill);

        content = content.push(scrollable);

        // Info footer
        let count = filtered_processes.len() as i32;
        let info = widget::text(fl!("process-count", count = count))
            .size(10);
        content = content.push(info);

        // Toast notification
        if let Some(ref toast) = self.toast {
            let toast_text = widget::text(&toast.message)
                .size(11);
            
            let toast_content = widget::row()
                .spacing(4)
                .align_y(Alignment::Center)
                .push(toast_text)
                .padding(8);
            
            content = content.push(toast_content);
        }

        self.core.applet.popup_container(content).into()
    }

    /// Register subscriptions for this application.
    fn subscription(&self) -> Subscription<Self::Message> {
        struct RefreshSubscription;

        Subscription::batch(vec![
            // Auto-refresh every 2 seconds
            Subscription::run_with_id(
                std::any::TypeId::of::<RefreshSubscription>(),
                cosmic::iced::stream::channel(4, move |mut channel| async move {
                    loop {
                        tokio::time::sleep(Duration::from_secs(2)).await;
                        _ = channel.send(Message::RefreshProcesses).await;
                    }
                }),
            ),
            // Watch for application configuration changes.
            self.core()
                .watch_config::<Config>(Self::APP_ID)
                .map(|update| Message::UpdateConfig(update.config)),
        ])
    }

    /// Handles messages emitted by the application and its widgets.
    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        match message {
            Message::SubscriptionChannel => {}
            Message::UpdateConfig(config) => {
                self.config = config;
            }
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
            Message::TogglePopup => {
                return if let Some(p) = self.popup.take() {
                    destroy_popup(p)
                } else {
                    self.refresh_processes();
                    let new_id = Id::unique();
                    self.popup.replace(new_id);
                    let mut popup_settings = self.core.applet.get_popup_settings(
                        self.core.main_window_id().unwrap(),
                        new_id,
                        None,
                        None,
                        None,
                    );
                    popup_settings.positioner.size_limits = Limits::NONE
                        .max_width(500.0)
                        .min_width(450.0)
                        .min_height(450.0)
                        .max_height(500.0);
                    get_popup(popup_settings)
                }
            }
            Message::PopupClosed(id) => {
                if self.popup.as_ref() == Some(&id) {
                    self.popup = None;
                }
            }
        }
        Task::none()
    }

    fn style(&self) -> Option<cosmic::iced_runtime::Appearance> {
        Some(cosmic::applet::style())
    }
}

impl AppModel {
    fn refresh_processes(&mut self) {
        let mut processes = self.process_manager.get_processes(self.sort_by);
        if !self.show_all {
            processes.truncate(10);
        }
        self.processes = processes;
    }

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

    fn create_process_row<'a>(&self, process: &'a ProcessInfo) -> Element<'a, Message> {
        let is_selected = self.selected_process.as_ref().map(|p| p.pid) == Some(process.pid);

        // Truncar nome se muito longo
        let display_name = if process.name.len() > 15 {
            format!("{}...", &process.name[..12])
        } else {
            process.name.clone()
        };

        let name_text = widget::text(display_name)
            .size(12)
            .width(Length::Fixed(85.0));

        let pid_text = widget::text(format!("{}", process.pid))
            .size(11)
            .width(Length::Fixed(60.0))
            .align_x(cosmic::iced::alignment::Horizontal::Center);

        let cpu_text = widget::text(format!("{:.0}%", process.cpu_usage))
            .size(11)
            .width(Length::Fixed(60.0))
            .align_x(cosmic::iced::alignment::Horizontal::Center);

        let mem_text = widget::text(format!("{:.0}MB", process.memory as f32 / 1024.0 / 1024.0))
            .size(11)
            .width(Length::Fixed(70.0))
            .align_x(cosmic::iced::alignment::Horizontal::Center);

        // Check if process can be killed
        let can_kill = self.process_manager.can_kill_process(process).is_ok();

        // Compact action buttons
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
                .spacing(2)
                .push(kill_button)
                .push(force_kill_button)
        } else {
            widget::row()
                .spacing(2)
                .push(
                    widget::button::icon(widget::icon::from_name("lock-symbolic"))
                        .padding(4)
                )
        };

        let info_row = widget::row()
            .spacing(4)
            .align_y(Alignment::Center)
            .push(name_text)
            .push(pid_text)
            .push(cpu_text)
            .push(mem_text)
            .push(widget::horizontal_space());

        let info_button = widget::button::custom(info_row)
            .on_press(Message::SelectProcess(Some(process.pid)))
            .padding([4, 0])
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
