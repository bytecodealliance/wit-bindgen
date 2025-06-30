//! Usage Tracking System for wit-bindgen
//!
//! This module implements a usage tracking system that:
//! - Assesses user skill level based on command usage patterns
//! - Provides personalized suggestions and help
//! - Tracks successful vs failed operations to improve recommendations
//! - Adapts error messages and help based on user expertise

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// User skill assessment levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillLevel {
    /// New to WebAssembly and WIT
    Beginner,
    /// Familiar with basic concepts
    Intermediate,
    /// Expert user with deep knowledge
    Advanced,
    /// Professional developer or framework author
    Expert,
}

impl Default for SkillLevel {
    fn default() -> Self {
        SkillLevel::Beginner
    }
}

/// Command usage patterns for skill assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandUsage {
    /// Number of times command was used
    pub count: u32,
    /// Number of successful executions
    pub successes: u32,
    /// Number of failed executions
    pub failures: u32,
    /// Average time between usage (for complexity assessment)
    pub frequency_score: f64,
}

impl Default for CommandUsage {
    fn default() -> Self {
        Self {
            count: 0,
            successes: 0,
            failures: 0,
            frequency_score: 0.0,
        }
    }
}

/// User behavior patterns and preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    /// Current assessed skill level
    pub skill_level: SkillLevel,
    /// Command usage statistics
    pub command_usage: HashMap<String, CommandUsage>,
    /// Preferred output format
    pub preferred_format: String,
    /// Whether user prefers verbose or concise help
    pub verbose_help: bool,
    /// Languages/frameworks user works with
    pub target_languages: Vec<String>,
    /// Common error patterns (for personalized suggestions)
    pub error_patterns: HashMap<String, u32>,
    /// Timestamp of last update
    pub last_updated: u64,
}

impl Default for UserProfile {
    fn default() -> Self {
        Self {
            skill_level: SkillLevel::Beginner,
            command_usage: HashMap::new(),
            preferred_format: "human".to_string(),
            verbose_help: true,
            target_languages: vec!["rust".to_string()],
            error_patterns: HashMap::new(),
            last_updated: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }
}

/// Usage tracking system manager
pub struct UsageTracker {
    profile: UserProfile,
    profile_path: PathBuf,
}

impl UsageTracker {
    /// Create new adaptive learning system
    pub fn new() -> Self {
        let profile_path = Self::get_profile_path();
        let profile = Self::load_profile(&profile_path).unwrap_or_default();

        Self {
            profile,
            profile_path,
        }
    }

    /// Get the path where user profile is stored
    fn get_profile_path() -> PathBuf {
        if let Some(config_dir) = dirs::config_dir() {
            config_dir.join("wit-bindgen").join("user_profile.json")
        } else {
            PathBuf::from(".wit-bindgen-profile.json")
        }
    }

    /// Load user profile from disk
    fn load_profile(path: &PathBuf) -> Result<UserProfile, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let profile: UserProfile = serde_json::from_str(&content)?;
        Ok(profile)
    }

    /// Save user profile to disk
    pub fn save_profile(&self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(parent) = self.profile_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(&self.profile)?;
        std::fs::write(&self.profile_path, content)?;
        Ok(())
    }

    /// Record command usage for skill assessment
    pub fn record_command_usage(&mut self, command: &str, success: bool) {
        let usage = self
            .profile
            .command_usage
            .entry(command.to_string())
            .or_default();
        usage.count += 1;

        if success {
            usage.successes += 1;
        } else {
            usage.failures += 1;
        }

        // Update frequency score (simple approach)
        usage.frequency_score = usage.count as f64
            / (std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
                - self.profile.last_updated) as f64;

        self.profile.last_updated = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Reassess skill level
        self.assess_skill_level();

        // Save profile
        let _ = self.save_profile();
    }

    /// Record error pattern for better suggestions
    pub fn record_error_pattern(&mut self, error_type: &str) {
        *self
            .profile
            .error_patterns
            .entry(error_type.to_string())
            .or_insert(0) += 1;
        let _ = self.save_profile();
    }

    /// Assess user skill level based on usage patterns
    fn assess_skill_level(&mut self) {
        let mut score = 0;

        // Advanced commands usage
        let advanced_commands = ["analyze", "deps", "help-ai"];
        for cmd in advanced_commands {
            if let Some(usage) = self.profile.command_usage.get(cmd) {
                score += usage.count as i32;
            }
        }

        // Success rate factor
        let total_commands: u32 = self.profile.command_usage.values().map(|u| u.count).sum();
        let total_successes: u32 = self
            .profile
            .command_usage
            .values()
            .map(|u| u.successes)
            .sum();

        if total_commands > 0 {
            let success_rate = total_successes as f64 / total_commands as f64;
            score += (success_rate * 50.0) as i32;
        }

        // Frequency of usage
        if total_commands > 20 {
            score += 20;
        } else if total_commands > 10 {
            score += 10;
        }

        // Update skill level
        self.profile.skill_level = match score {
            0..=20 => SkillLevel::Beginner,
            21..=50 => SkillLevel::Intermediate,
            51..=80 => SkillLevel::Advanced,
            _ => SkillLevel::Expert,
        };
    }

    /// Get current skill level
    pub fn get_skill_level(&self) -> SkillLevel {
        self.profile.skill_level
    }

    /// Get personalized help message based on skill level
    #[allow(dead_code)]
    pub fn get_personalized_help(&self, command: &str) -> String {
        match self.profile.skill_level {
            SkillLevel::Beginner => self.get_beginner_help(command),
            SkillLevel::Intermediate => self.get_intermediate_help(command),
            SkillLevel::Advanced => self.get_advanced_help(command),
            SkillLevel::Expert => self.get_expert_help(command),
        }
    }

    /// Get suggestions based on user's error patterns
    pub fn get_personalized_suggestions(&self, error_type: &str) -> Vec<String> {
        let mut suggestions = Vec::new();

        // Check if user frequently encounters this error
        if let Some(&count) = self.profile.error_patterns.get(error_type) {
            if count > 2 {
                suggestions.push(format!(
                    "You've encountered this '{}' error {} times before. Consider reviewing the documentation at: wit-bindgen help-ai",
                    error_type, count
                ));
            }
        }

        // Skill-level specific suggestions
        match self.profile.skill_level {
            SkillLevel::Beginner => {
                suggestions.extend(self.get_beginner_suggestions(error_type));
            }
            SkillLevel::Intermediate => {
                suggestions.extend(self.get_intermediate_suggestions(error_type));
            }
            SkillLevel::Advanced | SkillLevel::Expert => {
                suggestions.extend(self.get_advanced_suggestions(error_type));
            }
        }

        suggestions
    }

    /// Get user's preferred output format
    pub fn get_preferred_format(&self) -> &str {
        &self.profile.preferred_format
    }

    /// Update user preferences
    #[allow(dead_code)]
    pub fn update_preferences(&mut self, format: Option<&str>, verbose: Option<bool>) {
        if let Some(fmt) = format {
            self.profile.preferred_format = fmt.to_string();
        }
        if let Some(v) = verbose {
            self.profile.verbose_help = v;
        }
        let _ = self.save_profile();
    }

    /// Get usage statistics for reporting
    pub fn get_usage_stats(&self) -> HashMap<String, CommandUsage> {
        self.profile.command_usage.clone()
    }

    // Helper methods for skill-specific help
    fn get_beginner_help(&self, command: &str) -> String {
        match command {
            "validate" => {
                "BEGINNER TIP: The 'validate' command checks if your WIT files are correct.\n\
                 Try: wit-bindgen validate my-file.wit\n\
                 For automatic fixing: wit-bindgen validate --auto-deps my-file.wit"
                    .to_string()
            }
            "rust" => "BEGINNER TIP: The 'rust' command generates Rust bindings from WIT files.\n\
                 Try: wit-bindgen rust my-file.wit\n\
                 For enhanced documentation: wit-bindgen rust --intelligent-templates my-file.wit"
                .to_string(),
            _ => format!(
                "For detailed help with '{}', run: wit-bindgen help-ai",
                command
            ),
        }
    }

    fn get_intermediate_help(&self, command: &str) -> String {
        match command {
            "deps" => "INTERMEDIATE: Use deps commands for dependency management:\n\
                 - --sync-check: Verify deps/ matches imports\n\
                 - --add: Add new dependencies\n\
                 - --fix: Auto-fix dependency issues"
                .to_string(),
            "analyze" => "INTERMEDIATE: The analyze command provides deep insights:\n\
                 wit-bindgen analyze --format json file.wit\n\
                 Use JSON output for automation and tooling integration."
                .to_string(),
            _ => format!("Run 'wit-bindgen {} --help' for detailed options", command),
        }
    }

    fn get_advanced_help(&self, command: &str) -> String {
        match command {
            "help-ai" => {
                "ADVANCED: Access comprehensive AI documentation with structured schemas,\n\
                 command references, and integration patterns for automated workflows."
                    .to_string()
            }
            _ => format!(
                "Advanced usage: wit-bindgen {} with custom configurations",
                command
            ),
        }
    }

    fn get_expert_help(&self, command: &str) -> String {
        format!(
            "EXPERT: {} - Full control mode. See help-ai for API schemas",
            command
        )
    }

    // Helper methods for error-specific suggestions
    fn get_beginner_suggestions(&self, error_type: &str) -> Vec<String> {
        match error_type {
            "package_not_found" => vec![
                "BEGINNER: Package not found usually means missing dependencies.".to_string(),
                "Create a 'deps/' directory and add the missing packages there.".to_string(),
                "Use: wit-bindgen deps --add <package-name> --from <source>".to_string(),
            ],
            "parse_error" => vec![
                "BEGINNER: Syntax errors in WIT files are common.".to_string(),
                "Check for missing semicolons, braces, and correct keywords.".to_string(),
                "Use: wit-bindgen validate --analyze for detailed error info.".to_string(),
            ],
            _ => vec!["Check the documentation: wit-bindgen help-ai".to_string()],
        }
    }

    fn get_intermediate_suggestions(&self, error_type: &str) -> Vec<String> {
        match error_type {
            "package_not_found" => vec![
                "INTERMEDIATE: Verify dependency structure with --sync-check".to_string(),
                "Consider using --auto-deps for automatic resolution.".to_string(),
            ],
            _ => vec!["Use structured output: --format json for detailed diagnostics".to_string()],
        }
    }

    fn get_advanced_suggestions(&self, error_type: &str) -> Vec<String> {
        match error_type {
            "package_not_found" => vec![
                "ADVANCED: Check alphabetical ordering in deps/ directory.".to_string(),
                "Review wit-parser resolution algorithm in help-ai docs.".to_string(),
            ],
            _ => vec!["Consider custom error handling and automation scripts.".to_string()],
        }
    }
}

/// Global usage tracking system instance (thread-safe)
static USAGE_TRACKER: Lazy<Arc<Mutex<UsageTracker>>> =
    Lazy::new(|| Arc::new(Mutex::new(UsageTracker::new())));

/// Get a reference to the global usage tracking system
pub fn with_usage_tracker<R, F>(f: F) -> R
where
    F: FnOnce(&mut UsageTracker) -> R,
{
    let mut tracker = USAGE_TRACKER.lock().unwrap();
    f(&mut *tracker)
}

/// Convenience function for backwards compatibility
/// Note: This returns a clone of the Arc for thread safety
#[allow(dead_code)]
pub fn get_usage_tracker_handle() -> Arc<Mutex<UsageTracker>> {
    Arc::clone(&USAGE_TRACKER)
}
