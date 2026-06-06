use colored::Colorize;

use crate::shell::Shell;

impl Shell {
    pub(crate) fn handle_jobs_command(&self) {
        if !self.job_manager.has_jobs() {
            println!("{}", "No background jobs".cyan());
            return;
        }

        println!("{}", "Background jobs:".cyan());
        println!("{}  {}  {}", "ID".cyan(), "PID".cyan(), "Command".cyan());

        for job in self.job_manager.list_jobs() {
            let status_str = job.status.to_string().green();

            println!(
                "{}  {}  {}  {}",
                format!("[{}]", job.id).cyan(),
                job.pid,
                status_str,
                job.command
            );
        }
    }

    pub(crate) fn handle_fg_command(&mut self, args: &[String]) {
        if args.is_empty() {
            eprintln!("{} Job number required", "fg:".red());
            return;
        }

        let job_id = if args[0].starts_with('%') {
            args[0][1..].parse::<u32>()
        } else {
            args[0].parse::<u32>()
        };

        let job_id = match job_id {
            Ok(id) => id,
            Err(_) => {
                eprintln!("{} Invalid job number '{}'", "fg:".red(), args[0]);
                return;
            }
        };

        let job_index = match self.job_manager.find_job_index(job_id) {
            Some(index) => index,
            None => {
                eprintln!("{} Job %{} not found", "fg:".red(), job_id);
                return;
            }
        };

        let Some(job) = self.job_manager.get_job(job_id) else {
            eprintln!("{} Job %{} not found", "fg:".red(), job_id);
            return;
        };
        println!("{} [{}]", "Continuing job:".cyan(), job.id);

        let _ = self.job_manager.remove_job(job_index);
    }

    pub(crate) fn handle_bg_command(&mut self, args: &[String]) {
        if args.is_empty() {
            eprintln!("{} Job number required", "bg:".red());
            return;
        }

        let job_id = if args[0].starts_with('%') {
            args[0][1..].parse::<u32>()
        } else {
            args[0].parse::<u32>()
        };

        let job_id = match job_id {
            Ok(id) => id,
            Err(_) => {
                eprintln!("{} Invalid job number '{}'", "bg:".red(), args[0]);
                return;
            }
        };

        let _job_index = match self.job_manager.find_job_index(job_id) {
            Some(index) => index,
            None => {
                eprintln!("{} Job %{} not found", "bg:".red(), job_id);
                return;
            }
        };

        if let Some(job) = self.job_manager.get_job(job_id) {
            println!("{} [{}]", "Continue background job:".cyan(), job.id);
        }
    }
}
