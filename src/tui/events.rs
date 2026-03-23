use super::app::{ModuleContainer, TuiApp};
use super::canary_view;
use super::handlers;
use super::tabs::TabIndex;
use crossterm::event::KeyCode;

/// Possible result from handling a key event.
pub(crate) enum KeyAction {
    /// Continue the main loop.
    Continue,
    /// Quit the application.
    Quit,
}

/// Handle a keyboard event, mutating app state as needed.
/// Returns `KeyAction::Quit` if the application should exit.
///
/// Note: Some key handlers are async because they invoke module operations
/// (healer, autopolicy, etc.). This function is kept synchronous for the
/// majority of key handlers. Async operations are handled separately in
/// `handle_key_event_async`.
pub(crate) fn handle_key_event(app: &mut TuiApp, key_code: KeyCode) -> KeyAction {
    match key_code {
        KeyCode::Char('?') => {
            // Toggle help overlay
            app.show_help = !app.show_help;
        }
        KeyCode::Esc if app.show_help => {
            // Close help overlay
            app.show_help = false;
        }
        KeyCode::Char('q') if !app.show_help => return KeyAction::Quit,
        KeyCode::Tab if !app.show_help => {
            let count = TabIndex::count();
            app.selected_tab = (app.selected_tab + 1) % count;
        }
        KeyCode::BackTab if !app.show_help => {
            let count = TabIndex::count();
            app.selected_tab = if app.selected_tab == 0 {
                count - 1
            } else {
                app.selected_tab - 1
            };
        }
        KeyCode::Char('r') if !app.show_help && app.selected_tab == 9 => {
            // Refresh recordings from disk storage
            let count = match &mut app.modules {
                ModuleContainer::Enriched { replay, .. } => {
                    replay.list_recordings().map(|r| r.len())
                }
                ModuleContainer::Mock { replay, .. } => replay.list_recordings().map(|r| r.len()),
            };
            match count {
                Ok(n) => app.set_status_message(&format!("Refreshed: {} recordings found", n)),
                Err(e) => app.set_status_message(&format!("Refresh failed: {}", e)),
            }
        }
        KeyCode::Char('t')
            if !app.show_help && app.selected_tab == 9 && !app.replay_view.time_travel_mode =>
        {
            // Enter time-travel mode
            app.replay_view.enter_time_travel();
        }
        KeyCode::Esc
            if !app.show_help && app.selected_tab == 9 && app.replay_view.time_travel_mode =>
        {
            // Exit time-travel mode
            app.replay_view.exit_time_travel();
        }
        KeyCode::Left
            if !app.show_help && app.selected_tab == 9 && app.replay_view.time_travel_mode =>
        {
            // Step backward in timeline
            app.replay_view.step_backward();
        }
        KeyCode::Right
            if !app.show_help && app.selected_tab == 9 && app.replay_view.time_travel_mode =>
        {
            // Step forward in timeline
            app.replay_view.step_forward();
        }
        KeyCode::Char(' ')
            if !app.show_help && app.selected_tab == 9 && app.replay_view.time_travel_mode =>
        {
            // Toggle playback
            app.replay_view.toggle_playback();
        }
        KeyCode::Char('[')
            if !app.show_help && app.selected_tab == 9 && app.replay_view.time_travel_mode =>
        {
            // Jump to previous event
            app.replay_view.jump_to_prev_event();
        }
        KeyCode::Char(']')
            if !app.show_help && app.selected_tab == 9 && app.replay_view.time_travel_mode =>
        {
            // Jump to next event
            app.replay_view.jump_to_next_event();
        }
        KeyCode::Char('+')
            if !app.show_help && app.selected_tab == 9 && app.replay_view.time_travel_mode =>
        {
            // Increase playback speed
            app.replay_view.adjust_speed(true);
        }
        KeyCode::Char('-')
            if !app.show_help && app.selected_tab == 9 && app.replay_view.time_travel_mode =>
        {
            // Decrease playback speed
            app.replay_view.adjust_speed(false);
        }
        KeyCode::Up
            if !app.show_help && app.selected_tab == 9 && !app.replay_view.time_travel_mode =>
        {
            // Navigate recordings up
            app.replay_view.move_selection_up();
        }
        KeyCode::Down
            if !app.show_help && app.selected_tab == 9 && !app.replay_view.time_travel_mode =>
        {
            // Navigate recordings down (max 4 recordings in demo)
            app.replay_view.move_selection_down(4);
        }
        KeyCode::Char('v') if !app.show_help && app.selected_tab == 10 => {
            // Toggle between presets and active experiments
            app.chaos_view.toggle_view();
        }
        KeyCode::Up if !app.show_help && app.selected_tab == 10 => {
            // Navigate chaos experiments/presets up
            app.chaos_view.move_selection_up();
        }
        KeyCode::Down if !app.show_help && app.selected_tab == 10 => {
            // Navigate chaos experiments/presets down
            let max = if app.chaos_view.show_presets { 7 } else { 2 };
            app.chaos_view.move_selection_down(max);
        }
        KeyCode::Enter
            if !app.show_help
                && app.selected_tab == 10
                && app.chaos_view.show_presets
                && !app.chaos_view.confirmation_mode =>
        {
            // Run chaos experiment
            app.chaos_view.trigger_confirmation();
        }
        KeyCode::Char('y')
            if !app.show_help && app.selected_tab == 10 && app.chaos_view.confirmation_mode =>
        {
            // Confirm chaos experiment
            app.chaos_view.cancel_confirmation();
            app.set_status_message("Chaos experiment started!");
        }
        KeyCode::Char('n')
            if !app.show_help && app.selected_tab == 10 && app.chaos_view.confirmation_mode =>
        {
            // Cancel chaos experiment
            app.chaos_view.cancel_confirmation();
            app.set_status_message("Chaos experiment cancelled");
        }
        KeyCode::Char('s')
            if !app.show_help && app.selected_tab == 10 && !app.chaos_view.show_presets =>
        {
            // Stop selected experiment
            app.set_status_message("Chaos experiment stopped");
        }
        KeyCode::Char('S')
            if !app.show_help && app.selected_tab == 10 && !app.chaos_view.show_presets =>
        {
            // Stop all experiments
            app.set_status_message("All chaos experiments stopped");
        }
        KeyCode::Char('b')
            if !app.show_help
                && app.selected_tab == 10
                && !app.chaos_view.circuit_breaker_confirm =>
        {
            // Trigger circuit breaker
            app.chaos_view.circuit_breaker_confirm = true;
        }
        // Canary tab (11) keyboard handlers
        KeyCode::Up if !app.show_help && app.selected_tab == 11 => {
            app.canary_view.move_selection_up();
        }
        KeyCode::Down if !app.show_help && app.selected_tab == 11 => {
            app.canary_view.move_selection_down(2); // 2 active canaries
        }
        KeyCode::Char('p') if !app.show_help && app.selected_tab == 11 => {
            app.canary_view
                .trigger_confirmation(canary_view::ConfirmationType::Promote);
        }
        KeyCode::Char('r') if !app.show_help && app.selected_tab == 11 => {
            app.canary_view
                .trigger_confirmation(canary_view::ConfirmationType::Rollback);
        }
        KeyCode::Char('+') if !app.show_help && app.selected_tab == 11 => {
            app.set_status_message("Canary traffic increased by 10%");
        }
        KeyCode::Char('d') if !app.show_help && app.selected_tab == 11 => {
            app.canary_view.toggle_details();
        }
        KeyCode::Char('y')
            if !app.show_help
                && app.selected_tab == 11
                && app.canary_view.confirmation_mode != canary_view::ConfirmationType::None =>
        {
            app.canary_view.cancel_confirmation();
            app.set_status_message("Canary action confirmed");
        }
        KeyCode::Char('n')
            if !app.show_help
                && app.selected_tab == 11
                && app.canary_view.confirmation_mode != canary_view::ConfirmationType::None =>
        {
            app.canary_view.cancel_confirmation();
            app.set_status_message("Canary action cancelled");
        }
        // MultiCluster tab (12) keyboard handlers
        KeyCode::Up if !app.show_help && app.selected_tab == 12 => {
            app.multicluster_view.move_selection_up();
        }
        KeyCode::Down if !app.show_help && app.selected_tab == 12 => {
            app.multicluster_view.move_selection_down(4); // 4 clusters
        }
        KeyCode::Char('v') if !app.show_help && app.selected_tab == 12 => {
            app.multicluster_view.cycle_view();
        }
        KeyCode::Char('s') if !app.show_help && app.selected_tab == 8 => {
            // Run simulation
            app.simulator_view.trigger_simulation();
        }
        KeyCode::Char('c') if !app.show_help && app.selected_tab == 8 => {
            // Clear simulation results
            app.simulator_view.clear_simulation();
        }
        KeyCode::Up
            if !app.show_help
                && app.selected_tab == 8
                && app.simulator_view.last_simulation.is_none() =>
        {
            // Navigate scenarios up
            app.simulator_view.move_selection_up();
        }
        KeyCode::Down
            if !app.show_help
                && app.selected_tab == 8
                && app.simulator_view.last_simulation.is_none() =>
        {
            // Navigate scenarios down (max 7 scenarios)
            app.simulator_view.move_selection_down(7);
        }
        KeyCode::Char('d') if !app.show_help && app.selected_tab == 5 => {
            // Detect problems manually (Healer tab) - handled in async handler
            // This arm is intentionally empty; the async version handles it.
        }
        // Flows tab (0) keyboard handlers
        KeyCode::Up if !app.show_help && app.selected_tab == 0 && !app.show_packet_explanation => {
            // Navigate flows up
            if app.selected_flow_index > 0 {
                app.selected_flow_index -= 1;
            }
        }
        KeyCode::Down
            if !app.show_help && app.selected_tab == 0 && !app.show_packet_explanation =>
        {
            // Navigate flows down
            if app.selected_flow_index < app.flows.len().saturating_sub(1).min(49) {
                app.selected_flow_index += 1;
            }
        }
        KeyCode::Char('e') if !app.show_help && app.selected_tab == 0 => {
            // Toggle packet explanation
            app.show_packet_explanation = !app.show_packet_explanation;
            if app.show_packet_explanation && !app.flows.is_empty() {
                app.set_status_message("Explaining packet...");
            }
        }
        KeyCode::Esc if !app.show_help && app.selected_tab == 0 && app.show_packet_explanation => {
            // Exit packet explanation
            app.show_packet_explanation = false;
        }
        _ => {
            // Delegate to autopolicy and rootcause handlers
            if !handlers::handle_autopolicy_keys(app, key_code) {
                handlers::handle_rootcause_keys(app, key_code);
            }
        }
    }

    KeyAction::Continue
}

/// Handle async key events that require awaiting module operations.
/// This should be called after `handle_key_event` for keys that need async work.
pub(crate) async fn handle_key_event_async(app: &mut TuiApp, key_code: KeyCode) {
    match key_code {
        KeyCode::Char('d') if !app.show_help && app.selected_tab == 5 => {
            // Detect problems manually (Healer tab)
            match &mut app.modules {
                ModuleContainer::Enriched { healer, .. } => {
                    if let Err(e) = healer.run_enriched().await {
                        tracing::warn!("Healer enriched run failed: {}", e);
                    } else {
                        tracing::info!("Manual problem detection completed");
                    }
                }
                ModuleContainer::Mock { healer, .. } => {
                    if let Err(e) = healer.run().await {
                        tracing::warn!("Healer run failed: {}", e);
                    }
                }
            }
            app.last_healer_run = std::time::Instant::now();
        }
        KeyCode::Char('u') if !app.show_help && app.selected_tab == 6 => {
            // Update learning manually (AutoPolicy tab)
            match &mut app.modules {
                ModuleContainer::Enriched { autopolicy, .. } => {
                    if let Err(e) = autopolicy.update_enriched().await {
                        tracing::warn!("AutoPolicy enriched update failed: {}", e);
                    } else {
                        tracing::info!("Manual policy learning update completed");
                    }
                }
                ModuleContainer::Mock { autopolicy, .. } => {
                    if let Err(e) = autopolicy.update().await {
                        tracing::warn!("AutoPolicy update failed: {}", e);
                    }
                }
            }
            app.last_autopolicy_update = std::time::Instant::now();
        }
        _ => {}
    }
}
