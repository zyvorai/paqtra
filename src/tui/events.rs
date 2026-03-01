use crossterm::event::KeyCode;
use super::app::{TuiApp, ModuleContainer};
use super::canary_view;

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
            app.selected_tab = (app.selected_tab + 1) % 10;
        }
        KeyCode::BackTab if !app.show_help => {
            app.selected_tab = if app.selected_tab == 0 {
                9
            } else {
                app.selected_tab - 1
            };
        }
        KeyCode::Char('r') if !app.show_help && app.selected_tab == 9 => {
            // Refresh recordings list (placeholder)
        }
        KeyCode::Char('t') if !app.show_help && app.selected_tab == 9 && !app.replay_view.time_travel_mode => {
            // Enter time-travel mode
            app.replay_view.enter_time_travel();
        }
        KeyCode::Esc if !app.show_help && app.selected_tab == 9 && app.replay_view.time_travel_mode => {
            // Exit time-travel mode
            app.replay_view.exit_time_travel();
        }
        KeyCode::Left if !app.show_help && app.selected_tab == 9 && app.replay_view.time_travel_mode => {
            // Step backward in timeline
            app.replay_view.step_backward();
        }
        KeyCode::Right if !app.show_help && app.selected_tab == 9 && app.replay_view.time_travel_mode => {
            // Step forward in timeline
            app.replay_view.step_forward();
        }
        KeyCode::Char(' ') if !app.show_help && app.selected_tab == 9 && app.replay_view.time_travel_mode => {
            // Toggle playback
            app.replay_view.toggle_playback();
        }
        KeyCode::Char('[') if !app.show_help && app.selected_tab == 9 && app.replay_view.time_travel_mode => {
            // Jump to previous event
            app.replay_view.jump_to_prev_event();
        }
        KeyCode::Char(']') if !app.show_help && app.selected_tab == 9 && app.replay_view.time_travel_mode => {
            // Jump to next event
            app.replay_view.jump_to_next_event();
        }
        KeyCode::Char('+') if !app.show_help && app.selected_tab == 9 && app.replay_view.time_travel_mode => {
            // Increase playback speed
            app.replay_view.adjust_speed(true);
        }
        KeyCode::Char('-') if !app.show_help && app.selected_tab == 9 && app.replay_view.time_travel_mode => {
            // Decrease playback speed
            app.replay_view.adjust_speed(false);
        }
        KeyCode::Up if !app.show_help && app.selected_tab == 9 && !app.replay_view.time_travel_mode => {
            // Navigate recordings up
            app.replay_view.move_selection_up();
        }
        KeyCode::Down if !app.show_help && app.selected_tab == 9 && !app.replay_view.time_travel_mode => {
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
        KeyCode::Enter if !app.show_help && app.selected_tab == 10 && app.chaos_view.show_presets && !app.chaos_view.confirmation_mode => {
            // Run chaos experiment
            app.chaos_view.trigger_confirmation();
        }
        KeyCode::Char('y') if !app.show_help && app.selected_tab == 10 && app.chaos_view.confirmation_mode => {
            // Confirm chaos experiment
            app.chaos_view.cancel_confirmation();
            app.set_status_message("Chaos experiment started!");
        }
        KeyCode::Char('n') if !app.show_help && app.selected_tab == 10 && app.chaos_view.confirmation_mode => {
            // Cancel chaos experiment
            app.chaos_view.cancel_confirmation();
            app.set_status_message("Chaos experiment cancelled");
        }
        KeyCode::Char('s') if !app.show_help && app.selected_tab == 10 && !app.chaos_view.show_presets => {
            // Stop selected experiment
            app.set_status_message("Chaos experiment stopped");
        }
        KeyCode::Char('S') if !app.show_help && app.selected_tab == 10 && !app.chaos_view.show_presets => {
            // Stop all experiments
            app.set_status_message("All chaos experiments stopped");
        }
        KeyCode::Char('b') if !app.show_help && app.selected_tab == 10 && !app.chaos_view.circuit_breaker_confirm => {
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
            app.canary_view.trigger_confirmation(canary_view::ConfirmationType::Promote);
        }
        KeyCode::Char('r') if !app.show_help && app.selected_tab == 11 => {
            app.canary_view.trigger_confirmation(canary_view::ConfirmationType::Rollback);
        }
        KeyCode::Char('+') if !app.show_help && app.selected_tab == 11 => {
            app.set_status_message("Canary traffic increased by 10%");
        }
        KeyCode::Char('d') if !app.show_help && app.selected_tab == 11 => {
            app.canary_view.toggle_details();
        }
        KeyCode::Char('y') if !app.show_help && app.selected_tab == 11 && app.canary_view.confirmation_mode != canary_view::ConfirmationType::None => {
            app.canary_view.cancel_confirmation();
            app.set_status_message("Canary action confirmed");
        }
        KeyCode::Char('n') if !app.show_help && app.selected_tab == 11 && app.canary_view.confirmation_mode != canary_view::ConfirmationType::None => {
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
        KeyCode::Up if !app.show_help && app.selected_tab == 8 && app.simulator_view.last_simulation.is_none() => {
            // Navigate scenarios up
            app.simulator_view.move_selection_up();
        }
        KeyCode::Down if !app.show_help && app.selected_tab == 8 && app.simulator_view.last_simulation.is_none() => {
            // Navigate scenarios down (max 7 scenarios)
            app.simulator_view.move_selection_down(7);
        }
        KeyCode::Char('d') if !app.show_help && app.selected_tab == 5 => {
            // Detect problems manually (Healer tab) - handled in async handler
            // This arm is intentionally empty; the async version handles it.
        }
        KeyCode::Char('u') if !app.show_help && app.selected_tab == 6 => {
            // Update learning manually (AutoPolicy tab) - handled in async handler
            // This arm is intentionally empty; the async version handles it.
        }
        KeyCode::Char('g') if !app.show_help && app.selected_tab == 6 => {
            // Generate policies (AutoPolicy tab)
            let result = match &mut app.modules {
                ModuleContainer::Enriched { autopolicy, .. } => autopolicy.generate_policies(),
                ModuleContainer::Mock { autopolicy, .. } => autopolicy.generate_policies(),
            };

            match result {
                Ok(policies) => {
                    if policies.is_empty() {
                        app.set_status_message("No policies generated. Need more observations (min 10).");
                        tracing::info!("No policies generated - insufficient observations");
                    } else {
                        // Save policies to files
                        match app.save_policies(&policies) {
                            Ok(count) => {
                                app.set_status_message(&format!(
                                    "Generated {} policies -> saved to ./policies/",
                                    count
                                ));
                                tracing::info!("Generated and saved {} policies", count);
                            }
                            Err(e) => {
                                app.set_status_message(&format!("Error saving policies: {}", e));
                                tracing::warn!("Failed to save policies: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    app.set_status_message(&format!("Policy generation failed: {}", e));
                    tracing::warn!("Policy generation failed: {}", e);
                }
            }
        }
        KeyCode::Char('A') if !app.show_help && app.selected_tab == 6 && !app.policy_detail_mode => {
            // Batch apply all unapplied policies
            let policies = match &app.modules {
                ModuleContainer::Enriched { autopolicy, .. } => autopolicy.policies(),
                ModuleContainer::Mock { autopolicy, .. } => autopolicy.policies(),
            };

            let unapplied_count = policies.iter().filter(|p| !app.applied_policies.contains(&p.name)).count();

            if unapplied_count == 0 {
                app.set_status_message("All policies already applied");
            } else {
                app.policy_batch_apply_confirmation = true;
                app.set_status_message(&format!("Apply {} policies? Press 'y' to confirm, 'n' to cancel", unapplied_count));
            }
        }
        KeyCode::Char('R') if !app.show_help && app.selected_tab == 6 && !app.policy_detail_mode => {
            // Batch rollback all applied policies
            let applied_count = app.applied_policies.len();

            if applied_count == 0 {
                app.set_status_message("No policies applied to rollback");
            } else {
                app.policy_batch_rollback_confirmation = true;
                app.set_status_message(&format!("Rollback {} policies? Press 'y' to confirm, 'n' to cancel", applied_count));
            }
        }
        KeyCode::Char('y') if !app.show_help && app.selected_tab == 6 && app.policy_batch_apply_confirmation => {
            // Confirm batch apply
            app.policy_batch_apply_confirmation = false;

            let policies = match &app.modules {
                ModuleContainer::Enriched { autopolicy, .. } => autopolicy.policies(),
                ModuleContainer::Mock { autopolicy, .. } => autopolicy.policies(),
            };

            let mut applied = 0;
            let mut failed = 0;

            for policy in policies.iter() {
                if !app.applied_policies.contains(&policy.name) {
                    let policy_name = policy.name.clone();
                    let policy_yaml = policy.yaml.clone();

                    match app.apply_policy_kubectl(&policy_name, &policy_yaml) {
                        Ok(_) => {
                            app.applied_policies.insert(policy_name.clone());
                            applied += 1;
                            tracing::info!("Applied policy: {}", policy_name);
                        }
                        Err(e) => {
                            failed += 1;
                            tracing::error!("Failed to apply policy '{}': {}", policy_name, e);
                        }
                    }
                }
            }

            if failed == 0 {
                app.set_status_message(&format!("Applied {} policies successfully", applied));
            } else {
                app.set_status_message(&format!("Applied: {}, Failed: {}", applied, failed));
            }
        }
        KeyCode::Char('y') if !app.show_help && app.selected_tab == 6 && app.policy_batch_rollback_confirmation => {
            // Confirm batch rollback
            app.policy_batch_rollback_confirmation = false;

            let policies = match &app.modules {
                ModuleContainer::Enriched { autopolicy, .. } => autopolicy.policies(),
                ModuleContainer::Mock { autopolicy, .. } => autopolicy.policies(),
            };

            let mut rolled_back = 0;
            let mut failed = 0;
            let applied_names: Vec<String> = app.applied_policies.iter().cloned().collect();

            for policy_name in applied_names {
                // Find the policy to get its namespace
                if let Some(policy) = policies.iter().find(|p| p.name == policy_name) {
                    match app.rollback_policy_kubectl(&policy.name, &policy.namespace) {
                        Ok(_) => {
                            app.applied_policies.remove(&policy.name);
                            rolled_back += 1;
                            tracing::info!("Rolled back policy: {}", policy.name);
                        }
                        Err(e) => {
                            failed += 1;
                            tracing::error!("Failed to rollback policy '{}': {}", policy.name, e);
                        }
                    }
                }
            }

            if failed == 0 {
                app.set_status_message(&format!("Rolled back {} policies successfully", rolled_back));
            } else {
                app.set_status_message(&format!("Rolled back: {}, Failed: {}", rolled_back, failed));
            }
        }
        KeyCode::Char('n') if !app.show_help && app.selected_tab == 6 && (app.policy_batch_apply_confirmation || app.policy_batch_rollback_confirmation) => {
            // Cancel batch operation
            if app.policy_batch_apply_confirmation {
                app.policy_batch_apply_confirmation = false;
                app.set_status_message("Batch apply cancelled");
            } else if app.policy_batch_rollback_confirmation {
                app.policy_batch_rollback_confirmation = false;
                app.set_status_message("Batch rollback cancelled");
            }
        }
        KeyCode::Char('v') if !app.show_help && app.selected_tab == 6 => {
            // Toggle policy detail view (AutoPolicy tab)
            app.policy_detail_mode = !app.policy_detail_mode;
            if app.policy_detail_mode {
                app.set_status_message("Policy detail view (Up/Down: navigate, Esc: exit)");
            } else {
                app.selected_policy_index = 0;
            }
        }
        KeyCode::Up if !app.show_help && app.selected_tab == 6 && app.policy_detail_mode => {
            // Navigate policies up
            if app.selected_policy_index > 0 {
                app.selected_policy_index -= 1;
            }
        }
        KeyCode::Down if !app.show_help && app.selected_tab == 6 && app.policy_detail_mode => {
            // Navigate policies down
            let policy_count = match &app.modules {
                ModuleContainer::Enriched { autopolicy, .. } => autopolicy.policies().len(),
                ModuleContainer::Mock { autopolicy, .. } => autopolicy.policies().len(),
            };
            if app.selected_policy_index + 1 < policy_count {
                app.selected_policy_index += 1;
            }
        }
        KeyCode::Esc if !app.show_help && app.selected_tab == 6 => {
            // Exit or cancel confirmations
            if app.policy_apply_confirmation {
                app.policy_apply_confirmation = false;
                app.set_status_message("Policy application cancelled");
            } else if app.policy_rollback_confirmation {
                app.policy_rollback_confirmation = false;
                app.set_status_message("Policy rollback cancelled");
            } else if app.policy_batch_apply_confirmation {
                app.policy_batch_apply_confirmation = false;
                app.set_status_message("Batch apply cancelled");
            } else if app.policy_batch_rollback_confirmation {
                app.policy_batch_rollback_confirmation = false;
                app.set_status_message("Batch rollback cancelled");
            } else if app.policy_detail_mode {
                app.policy_detail_mode = false;
                app.selected_policy_index = 0;
            }
        }
        KeyCode::Char('a') if !app.show_help && app.selected_tab == 6 && app.policy_detail_mode && !app.policy_apply_confirmation => {
            // Trigger policy application confirmation
            let policies = match &app.modules {
                ModuleContainer::Enriched { autopolicy, .. } => autopolicy.policies(),
                ModuleContainer::Mock { autopolicy, .. } => autopolicy.policies(),
            };

            if !policies.is_empty() {
                let policy_name = &policies[app.selected_policy_index].name;
                if app.applied_policies.contains(policy_name) {
                    app.set_status_message(&format!("Policy '{}' already applied", policy_name));
                } else {
                    app.policy_apply_confirmation = true;
                    app.set_status_message(&format!("Apply policy '{}'? Press 'y' to confirm, 'n' to cancel", policy_name));
                }
            }
        }
        KeyCode::Char('y') if !app.show_help && app.selected_tab == 6 && app.policy_apply_confirmation => {
            // Confirm and apply policy
            app.policy_apply_confirmation = false;

            let policies = match &app.modules {
                ModuleContainer::Enriched { autopolicy, .. } => autopolicy.policies(),
                ModuleContainer::Mock { autopolicy, .. } => autopolicy.policies(),
            };

            if !policies.is_empty() {
                let policy = &policies[app.selected_policy_index];
                // Clone policy data to avoid borrow checker issues
                let policy_name = policy.name.clone();
                let policy_yaml = policy.yaml.clone();

                match app.apply_policy_kubectl(&policy_name, &policy_yaml) {
                    Ok(_) => {
                        app.applied_policies.insert(policy_name.clone());
                        app.set_status_message(&format!("Policy '{}' applied successfully", policy_name));
                        tracing::info!("Applied policy: {}", policy_name);
                    }
                    Err(e) => {
                        app.set_status_message(&format!("Failed to apply policy: {}", e));
                        tracing::error!("Failed to apply policy '{}': {}", policy_name, e);
                    }
                }
            }
        }
        KeyCode::Char('n') if !app.show_help && app.selected_tab == 6 && app.policy_apply_confirmation => {
            // Cancel policy application
            app.policy_apply_confirmation = false;
            app.set_status_message("Policy application cancelled");
        }
        KeyCode::Char('r') if !app.show_help && app.selected_tab == 6 && app.policy_detail_mode && !app.policy_apply_confirmation && !app.policy_rollback_confirmation => {
            // Trigger policy rollback confirmation
            let policies = match &app.modules {
                ModuleContainer::Enriched { autopolicy, .. } => autopolicy.policies(),
                ModuleContainer::Mock { autopolicy, .. } => autopolicy.policies(),
            };

            if !policies.is_empty() {
                let policy_name = &policies[app.selected_policy_index].name;
                if !app.applied_policies.contains(policy_name) {
                    app.set_status_message(&format!("Policy '{}' not applied, cannot rollback", policy_name));
                } else {
                    app.policy_rollback_confirmation = true;
                    app.set_status_message(&format!("Rollback policy '{}'? Press 'y' to confirm, 'n' to cancel", policy_name));
                }
            }
        }
        KeyCode::Char('y') if !app.show_help && app.selected_tab == 6 && app.policy_rollback_confirmation => {
            // Confirm and rollback policy
            app.policy_rollback_confirmation = false;

            let policies = match &app.modules {
                ModuleContainer::Enriched { autopolicy, .. } => autopolicy.policies(),
                ModuleContainer::Mock { autopolicy, .. } => autopolicy.policies(),
            };

            if !policies.is_empty() {
                let policy = &policies[app.selected_policy_index];
                let policy_name = policy.name.clone();
                let policy_namespace = policy.namespace.clone();

                match app.rollback_policy_kubectl(&policy_name, &policy_namespace) {
                    Ok(_) => {
                        app.applied_policies.remove(&policy_name);
                        app.set_status_message(&format!("Policy '{}' rolled back successfully", policy_name));
                        tracing::info!("Rolled back policy: {}", policy_name);
                    }
                    Err(e) => {
                        app.set_status_message(&format!("Failed to rollback policy: {}", e));
                        tracing::error!("Failed to rollback policy '{}': {}", policy_name, e);
                    }
                }
            }
        }
        KeyCode::Char('n') if !app.show_help && app.selected_tab == 6 && app.policy_rollback_confirmation => {
            // Cancel policy rollback
            app.policy_rollback_confirmation = false;
            app.set_status_message("Policy rollback cancelled");
        }
        KeyCode::Up if !app.show_help && app.selected_tab == 7 => {
            // Navigate fixes up
            if app.selected_fix_index > 0 {
                app.selected_fix_index -= 1;
            }
        }
        KeyCode::Down if !app.show_help && app.selected_tab == 7 => {
            // Navigate fixes down
            // Note: Max 4 fixes (hardcoded for now)
            if app.selected_fix_index < 3 {
                app.selected_fix_index += 1;
            }
        }
        KeyCode::Char('a') if !app.show_help && app.selected_tab == 7 && !app.fix_apply_confirmation => {
            // Trigger fix application confirmation
            app.fix_apply_confirmation = true;
            let fix_names = vec![
                "allow-8080 policy",
                "DNS egress policy",
                "MTU adjustment",
                "DB access policy"
            ];
            app.set_status_message(&format!(
                "Apply fix '{}'? Press 'y' to confirm, 'n' to cancel",
                fix_names[app.selected_fix_index]
            ));
        }
        KeyCode::Char('y') if !app.show_help && app.selected_tab == 7 && app.fix_apply_confirmation => {
            // Confirm and apply fix
            app.fix_apply_confirmation = false;

            let (fix_name, policy_yaml) = app.get_fix_policy(app.selected_fix_index);

            match app.apply_policy_kubectl(&fix_name, &policy_yaml) {
                Ok(_) => {
                    app.set_status_message(&format!("Applied fix: {}", fix_name));
                    tracing::info!("Applied RootCause fix policy: {}", fix_name);
                }
                Err(e) => {
                    app.set_status_message(&format!("Failed to apply fix: {}", e));
                    tracing::error!("Failed to apply RootCause fix '{}': {}", fix_name, e);
                }
            }
        }
        KeyCode::Char('n') if !app.show_help && app.selected_tab == 7 && app.fix_apply_confirmation => {
            // Cancel fix application
            app.fix_apply_confirmation = false;
            app.set_status_message("Fix application cancelled");
        }
        KeyCode::Esc if !app.show_help && app.selected_tab == 7 && app.fix_apply_confirmation => {
            // Cancel fix application with Esc
            app.fix_apply_confirmation = false;
            app.set_status_message("Fix application cancelled");
        }
        KeyCode::Up if !app.show_help && app.selected_tab == 0 && !app.show_packet_explanation => {
            // Navigate flows up
            if app.selected_flow_index > 0 {
                app.selected_flow_index -= 1;
            }
        }
        KeyCode::Down if !app.show_help && app.selected_tab == 0 && !app.show_packet_explanation => {
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
        _ => {}
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
