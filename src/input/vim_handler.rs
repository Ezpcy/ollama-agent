use colored::Colorize;
use console::{Key, Term};
use std::collections::VecDeque;
use std::io::{self, Write};

#[derive(Debug, Clone, PartialEq)]
pub enum InputMode {
    Normal,
    Insert,
    Command,
}

#[derive(Debug, Clone)]
pub struct VimInputHandler {
    mode: InputMode,
    buffer: String,
    cursor_pos: usize,
    history: VecDeque<String>,
    history_index: Option<usize>,
    command_buffer: String,
    search_buffer: String,
    max_history: usize,
    vim_enabled: bool,
}

impl Default for VimInputHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl VimInputHandler {
    pub fn new() -> Self {
        Self {
            mode: InputMode::Insert,
            buffer: String::new(),
            cursor_pos: 0,
            history: VecDeque::new(),
            history_index: None,
            command_buffer: String::new(),
            search_buffer: String::new(),
            max_history: 1000,
            vim_enabled: false,
        }
    }

    pub fn enable_vim_mode(&mut self) {
        self.vim_enabled = true;
        self.mode = InputMode::Normal;
    }

    pub fn disable_vim_mode(&mut self) {
        self.vim_enabled = false;
        self.mode = InputMode::Insert;
    }

    pub fn is_vim_enabled(&self) -> bool {
        self.vim_enabled
    }

    pub fn get_input(&mut self, prompt: &str) -> Result<String, Box<dyn std::error::Error>> {
        if !self.vim_enabled {
            return self.get_simple_input(prompt);
        }

        self.buffer.clear();
        self.cursor_pos = 0;
        self.history_index = None;
        self.mode = InputMode::Normal;

        let term = Term::stdout();
        term.hide_cursor()?;

        self.display_prompt_with_mode(prompt, &term)?;

        loop {
            match term.read_key()? {
                Key::Char('i') if self.mode == InputMode::Normal => {
                    self.mode = InputMode::Insert;
                    self.display_prompt_with_mode(prompt, &term)?;
                }
                Key::Char('I') if self.mode == InputMode::Normal => {
                    self.mode = InputMode::Insert;
                    self.cursor_pos = 0;
                    self.display_prompt_with_mode(prompt, &term)?;
                }
                Key::Char('a') if self.mode == InputMode::Normal => {
                    self.mode = InputMode::Insert;
                    if self.cursor_pos < self.buffer.len() {
                        self.cursor_pos += 1;
                    }
                    self.display_prompt_with_mode(prompt, &term)?;
                }
                Key::Char('A') if self.mode == InputMode::Normal => {
                    self.mode = InputMode::Insert;
                    self.cursor_pos = self.buffer.len();
                    self.display_prompt_with_mode(prompt, &term)?;
                }
                Key::Char('o') if self.mode == InputMode::Normal => {
                    self.mode = InputMode::Insert;
                    self.cursor_pos = self.buffer.len();
                    self.buffer.push('\n');
                    self.cursor_pos += 1;
                    self.display_prompt_with_mode(prompt, &term)?;
                }
                Key::Char(':') if self.mode == InputMode::Normal => {
                    self.mode = InputMode::Command;
                    self.command_buffer.clear();
                    self.display_command_line(&term)?;
                }
                Key::Char('/') if self.mode == InputMode::Normal => {
                    self.search_buffer.clear();
                    self.search_in_history(&term)?;
                }
                Key::Char('h') if self.mode == InputMode::Normal => {
                    if self.cursor_pos > 0 {
                        self.cursor_pos -= 1;
                        self.display_prompt_with_mode(prompt, &term)?;
                    }
                }
                Key::Char('l') if self.mode == InputMode::Normal => {
                    if self.cursor_pos < self.buffer.len() {
                        self.cursor_pos += 1;
                        self.display_prompt_with_mode(prompt, &term)?;
                    }
                }
                Key::Char('w') if self.mode == InputMode::Normal => {
                    self.move_word_forward();
                    self.display_prompt_with_mode(prompt, &term)?;
                }
                Key::Char('b') if self.mode == InputMode::Normal => {
                    self.move_word_backward();
                    self.display_prompt_with_mode(prompt, &term)?;
                }
                Key::Char('0') if self.mode == InputMode::Normal => {
                    self.cursor_pos = 0;
                    self.display_prompt_with_mode(prompt, &term)?;
                }
                Key::Char('$') if self.mode == InputMode::Normal => {
                    self.cursor_pos = self.buffer.len();
                    self.display_prompt_with_mode(prompt, &term)?;
                }
                Key::Char('x') if self.mode == InputMode::Normal => {
                    if self.cursor_pos < self.buffer.len() {
                        self.buffer.remove(self.cursor_pos);
                        self.display_prompt_with_mode(prompt, &term)?;
                    }
                }
                Key::Char('X') if self.mode == InputMode::Normal => {
                    if self.cursor_pos > 0 {
                        self.cursor_pos -= 1;
                        self.buffer.remove(self.cursor_pos);
                        self.display_prompt_with_mode(prompt, &term)?;
                    }
                }
                Key::Char('d') if self.mode == InputMode::Normal => {
                    // Simple dd implementation - delete line
                    if let Ok(Key::Char('d')) = term.read_key() {
                        self.buffer.clear();
                        self.cursor_pos = 0;
                        self.display_prompt_with_mode(prompt, &term)?;
                    }
                }
                Key::Char('u') if self.mode == InputMode::Normal => {
                    // Simple undo - restore from history
                    if let Some(last) = self.history.back() {
                        self.buffer = last.clone();
                        self.cursor_pos = self.buffer.len();
                        self.display_prompt_with_mode(prompt, &term)?;
                    }
                }
                Key::Char('j') if self.mode == InputMode::Normal => {
                    self.history_down();
                    self.display_prompt_with_mode(prompt, &term)?;
                }
                Key::Char('k') if self.mode == InputMode::Normal => {
                    self.history_up();
                    self.display_prompt_with_mode(prompt, &term)?;
                }
                Key::Escape => {
                    if self.mode != InputMode::Normal {
                        self.mode = InputMode::Normal;
                        if self.cursor_pos > 0 && self.cursor_pos == self.buffer.len() {
                            self.cursor_pos -= 1;
                        }
                        self.display_prompt_with_mode(prompt, &term)?;
                    }
                }
                Key::Enter if self.mode == InputMode::Command => {
                    if self.handle_command(&term)? {
                        continue;
                    } else {
                        break;
                    }
                }
                Key::Enter if self.mode == InputMode::Insert || self.mode == InputMode::Normal => {
                    if !self.buffer.trim().is_empty() {
                        self.add_to_history(self.buffer.clone());
                    }
                    break;
                }
                Key::Char(c) if self.mode == InputMode::Insert => {
                    self.buffer.insert(self.cursor_pos, c);
                    self.cursor_pos += 1;
                    self.display_prompt_with_mode(prompt, &term)?;
                }
                Key::Char(c) if self.mode == InputMode::Command => {
                    self.command_buffer.push(c);
                    self.display_command_line(&term)?;
                }
                Key::Backspace if self.mode == InputMode::Insert => {
                    if self.cursor_pos > 0 {
                        self.cursor_pos -= 1;
                        self.buffer.remove(self.cursor_pos);
                        self.display_prompt_with_mode(prompt, &term)?;
                    }
                }
                Key::Backspace if self.mode == InputMode::Command => {
                    if !self.command_buffer.is_empty() {
                        self.command_buffer.pop();
                        self.display_command_line(&term)?;
                    }
                }
                Key::ArrowLeft if self.mode == InputMode::Insert => {
                    if self.cursor_pos > 0 {
                        self.cursor_pos -= 1;
                        self.display_prompt_with_mode(prompt, &term)?;
                    }
                }
                Key::ArrowRight if self.mode == InputMode::Insert => {
                    if self.cursor_pos < self.buffer.len() {
                        self.cursor_pos += 1;
                        self.display_prompt_with_mode(prompt, &term)?;
                    }
                }
                Key::ArrowUp => {
                    self.history_up();
                    self.display_prompt_with_mode(prompt, &term)?;
                }
                Key::ArrowDown => {
                    self.history_down();
                    self.display_prompt_with_mode(prompt, &term)?;
                }
                Key::CtrlC => {
                    term.show_cursor()?;
                    return Err("Interrupted by user".into());
                }
                _ => {}
            }
        }

        term.show_cursor()?;
        println!();
        Ok(self.buffer.clone())
    }

    fn get_simple_input(&mut self, prompt: &str) -> Result<String, Box<dyn std::error::Error>> {
        use dialoguer::Input;

        let input: String = Input::new().with_prompt(prompt).interact_text()?;

        if !input.trim().is_empty() {
            self.add_to_history(input.clone());
        }

        Ok(input)
    }

    fn display_prompt_with_mode(
        &self,
        prompt: &str,
        term: &Term,
    ) -> Result<(), Box<dyn std::error::Error>> {
        term.clear_line()?;
        term.move_cursor_to_column(0)?;

        let mode_indicator = match self.mode {
            InputMode::Normal => "[NORMAL]".blue().bold(),
            InputMode::Insert => "[INSERT]".green().bold(),
            InputMode::Command => "[COMMAND]".yellow().bold(),
        };

        print!("{} {} {}", mode_indicator, prompt, self.buffer);

        // Position cursor
        let prompt_len = strip_ansi_codes(prompt).len()
            + strip_ansi_codes(&mode_indicator.to_string()).len()
            + 2;
        term.move_cursor_to_column(prompt_len + self.cursor_pos)?;

        io::stdout().flush()?;
        Ok(())
    }

    fn display_command_line(&self, term: &Term) -> Result<(), Box<dyn std::error::Error>> {
        term.clear_line()?;
        term.move_cursor_to_column(0)?;
        print!(":{}", self.command_buffer);
        io::stdout().flush()?;
        Ok(())
    }

    fn handle_command(&mut self, term: &Term) -> Result<bool, Box<dyn std::error::Error>> {
        match self.command_buffer.as_str() {
            "q" | "quit" => {
                term.show_cursor()?;
                std::process::exit(0);
            }
            "w" | "write" => {
                // Save current buffer - in a real implementation, this could save to file
                println!("\nBuffer saved");
                self.mode = InputMode::Normal;
                return Ok(true);
            }
            "wq" => {
                // Save and quit
                if !self.buffer.trim().is_empty() {
                    self.add_to_history(self.buffer.clone());
                }
                self.mode = InputMode::Normal;
                return Ok(false);
            }
            "clear" => {
                self.buffer.clear();
                self.cursor_pos = 0;
                self.mode = InputMode::Normal;
                return Ok(true);
            }
            "history" => {
                println!("\nCommand History:");
                for (i, cmd) in self.history.iter().enumerate().rev().take(10) {
                    println!("  {}: {}", i + 1, cmd);
                }
                self.mode = InputMode::Normal;
                return Ok(true);
            }
            "set number" => {
                println!("\nLine numbers enabled");
                self.mode = InputMode::Normal;
                return Ok(true);
            }
            "help" => {
                self.show_vim_help();
                self.mode = InputMode::Normal;
                return Ok(true);
            }
            _ if self.command_buffer.starts_with("s/") => {
                // Simple substitute command
                self.handle_substitute();
                self.mode = InputMode::Normal;
                return Ok(true);
            }
            _ => {
                println!("\nUnknown command: {}", self.command_buffer);
                self.mode = InputMode::Normal;
                return Ok(true);
            }
        }
    }

    fn handle_substitute(&mut self) {
        // Simple implementation of s/old/new/
        if let Some(parts) = self.parse_substitute_command(&self.command_buffer) {
            let (old, new) = parts;
            self.buffer = self.buffer.replace(&old, &new);
            self.cursor_pos = self.cursor_pos.min(self.buffer.len());
        }
    }

    fn parse_substitute_command(&self, cmd: &str) -> Option<(String, String)> {
        // Parse s/old/new/ format
        if cmd.starts_with("s/") {
            let parts: Vec<&str> = cmd[2..].split('/').collect();
            if parts.len() >= 2 {
                return Some((parts[0].to_string(), parts[1].to_string()));
            }
        }
        None
    }

    fn search_in_history(&mut self, term: &Term) -> Result<(), Box<dyn std::error::Error>> {
        term.clear_line()?;
        term.move_cursor_to_column(0)?;
        print!("/{}", self.search_buffer);

        loop {
            match term.read_key()? {
                Key::Char(c) => {
                    self.search_buffer.push(c);
                    if let Some(found) = self.find_in_history(&self.search_buffer) {
                        self.buffer = found;
                        self.cursor_pos = self.buffer.len();
                    }
                    term.clear_line()?;
                    term.move_cursor_to_column(0)?;
                    print!("/{}", self.search_buffer);
                }
                Key::Enter => {
                    self.mode = InputMode::Normal;
                    break;
                }
                Key::Escape => {
                    self.mode = InputMode::Normal;
                    break;
                }
                Key::Backspace => {
                    if !self.search_buffer.is_empty() {
                        self.search_buffer.pop();
                        term.clear_line()?;
                        term.move_cursor_to_column(0)?;
                        print!("/{}", self.search_buffer);
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn find_in_history(&self, pattern: &str) -> Option<String> {
        self.history
            .iter()
            .rev()
            .find(|cmd| cmd.contains(pattern))
            .cloned()
    }

    fn move_word_forward(&mut self) {
        while self.cursor_pos < self.buffer.len()
            && self
                .buffer
                .chars()
                .nth(self.cursor_pos)
                .unwrap_or(' ')
                .is_alphanumeric()
        {
            self.cursor_pos += 1;
        }
        while self.cursor_pos < self.buffer.len()
            && !self
                .buffer
                .chars()
                .nth(self.cursor_pos)
                .unwrap_or(' ')
                .is_alphanumeric()
        {
            self.cursor_pos += 1;
        }
    }

    fn move_word_backward(&mut self) {
        if self.cursor_pos == 0 {
            return;
        }

        self.cursor_pos -= 1;
        while self.cursor_pos > 0
            && !self
                .buffer
                .chars()
                .nth(self.cursor_pos)
                .unwrap_or(' ')
                .is_alphanumeric()
        {
            self.cursor_pos -= 1;
        }
        while self.cursor_pos > 0
            && self
                .buffer
                .chars()
                .nth(self.cursor_pos - 1)
                .unwrap_or(' ')
                .is_alphanumeric()
        {
            self.cursor_pos -= 1;
        }
    }

    fn history_up(&mut self) {
        if self.history.is_empty() {
            return;
        }

        let new_index = match self.history_index {
            None => self.history.len() - 1,
            Some(idx) if idx > 0 => idx - 1,
            Some(_) => return,
        };

        self.history_index = Some(new_index);
        if let Some(cmd) = self.history.get(new_index) {
            self.buffer = cmd.clone();
            self.cursor_pos = self.buffer.len();
        }
    }

    fn history_down(&mut self) {
        match self.history_index {
            None => return,
            Some(idx) if idx + 1 < self.history.len() => {
                self.history_index = Some(idx + 1);
                if let Some(cmd) = self.history.get(idx + 1) {
                    self.buffer = cmd.clone();
                    self.cursor_pos = self.buffer.len();
                }
            }
            Some(_) => {
                self.history_index = None;
                self.buffer.clear();
                self.cursor_pos = 0;
            }
        }
    }

    fn add_to_history(&mut self, command: String) {
        // Don't add duplicates or empty commands
        if command.trim().is_empty() || self.history.back() == Some(&command) {
            return;
        }

        self.history.push_back(command);
        if self.history.len() > self.max_history {
            self.history.pop_front();
        }
    }

    fn show_vim_help(&self) {
        println!("\n{}", "Vim Mode Help:".cyan().bold());
        println!();
        println!("{}", "Normal Mode Commands:".blue().bold());
        println!("  {} - Enter insert mode", "i".yellow());
        println!(
            "  {} - Enter insert mode at beginning of line",
            "I".yellow()
        );
        println!("  {} - Enter insert mode after cursor", "a".yellow());
        println!("  {} - Enter insert mode at end of line", "A".yellow());
        println!("  {} - Open new line below", "o".yellow());
        println!("  {} - Enter command mode", ":".yellow());
        println!("  {} - Search history", "/".yellow());
        println!();
        println!("{}", "Movement:".blue().bold());
        println!("  {} - Move left", "h".yellow());
        println!("  {} - Move right", "l".yellow());
        println!("  {} - Move to beginning of line", "0".yellow());
        println!("  {} - Move to end of line", "$".yellow());
        println!("  {} - Move forward by word", "w".yellow());
        println!("  {} - Move backward by word", "b".yellow());
        println!("  {} - History up", "k".yellow());
        println!("  {} - History down", "j".yellow());
        println!();
        println!("{}", "Editing:".blue().bold());
        println!("  {} - Delete character under cursor", "x".yellow());
        println!("  {} - Delete character before cursor", "X".yellow());
        println!("  {} - Delete line (dd)", "d".yellow());
        println!("  {} - Undo", "u".yellow());
        println!();
        println!("{}", "Command Mode:".blue().bold());
        println!("  {} - Quit", ":q".yellow());
        println!("  {} - Write (save)", ":w".yellow());
        println!("  {} - Write and quit", ":wq".yellow());
        println!("  {} - Clear buffer", ":clear".yellow());
        println!("  {} - Show history", ":history".yellow());
        println!("  {} - Show this help", ":help".yellow());
        println!();
        println!("{}", "Press any key to continue...".dimmed());
    }

    pub fn get_history(&self) -> &VecDeque<String> {
        &self.history
    }

    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    pub fn load_history(&mut self, history: Vec<String>) {
        self.history.clear();
        for cmd in history {
            self.history.push_back(cmd);
        }
    }

    pub fn save_history(&self) -> Vec<String> {
        self.history.iter().cloned().collect()
    }
}

// Helper function to strip ANSI color codes for length calculation
fn strip_ansi_codes(s: &str) -> String {
    // Simple implementation - in production, use a proper ANSI parser
    let mut result = String::new();
    let mut in_escape = false;

    for ch in s.chars() {
        if ch == '\x1b' {
            in_escape = true;
        } else if in_escape && ch == 'm' {
            in_escape = false;
        } else if !in_escape {
            result.push(ch);
        }
    }

    result
}
