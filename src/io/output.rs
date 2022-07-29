use std::rc::Rc;
use std::sync::Mutex;

use indicatif::ProgressBar;
use indicatif::ProgressStyle;
use lazy_static::lazy_static;

use crate::solver;
use crate::types;

pub fn solver_item_as_grid(constraint: &types::Constraint, item: &solver::Output) -> String {
    match item {
        solver::Output::Solution(solution) => solution_as_grid(constraint, solution),
        solver::Output::Guesses(fixed_values) => fixed_values_as_grid(constraint, fixed_values),
        solver::Output::Empty => String::new(),
    }
}

fn solution_as_grid(constraint: &types::Constraint, solution: &types::Solution) -> String {
    render_grid(
        constraint,
        &solution.iter().map(|&v| Some(v)).collect::<Vec<_>>(),
    )
}

pub fn fixed_values_as_grid(
    constraint: &types::Constraint,
    fixed_values: &types::FixedValues,
) -> String {
    let shape = &constraint.shape;
    let mut grid = vec![None; shape.num_cells];
    for (cell, value) in fixed_values {
        grid[*cell] = Some(*value);
    }
    render_grid(constraint, &grid)
}

fn render_grid(constraint: &types::Constraint, grid: &[Option<types::CellValue>]) -> String {
    let mut output = String::new();

    let shape = &constraint.shape;
    assert_eq!(shape.num_cells, grid.len());

    let pad_size = shape.num_values.to_string().len() + 1;

    for r in 0..shape.side_len {
        for c in 0..shape.side_len {
            let index = shape.make_cell_index(r, c);
            let display = match grid[index] {
                None => ".".to_string(),
                Some(v) => v.to_string(),
            };
            (0..pad_size - display.len()).for_each(|_| output.push(' '));
            output.push_str(&display);
        }
        output.push('\n');
    }

    output
}

pub fn solution_compact(solution: &types::Solution) -> String {
    format!(
        "[{}]",
        solution
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(" ")
    )
}

pub fn with_progress_bar<F: FnOnce(Rc<ProgressBar>)>(scale: u64, f: F) {
    let bar = Rc::new(ProgressBar::new(scale));
    bar.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {wide_bar:cyan/blue} {percent}%\n{wide_msg}"),
    );
    bar.enable_steady_tick(1000);
    bar.set_position(0);
    bar.set_message("Initializing...");

    f(bar.clone());

    bar.set_style(ProgressStyle::default_bar().template("[{elapsed_precise}] {msg}"));
    bar.finish();
}

pub fn print_above_progress_bar(output: &str) {
    if output.is_empty() {
        return;
    }

    if atty::is(atty::Stream::Stdout) {
        // We only need to worry about the bar if stdout is going to a tty.

        // Erase the bar (two lines).
        eprint!("\x1b[A\x1b[2K"); // Erase line above.
        eprint!("\x1b[A\x1b[2K"); // Erase line above.
        eprint!("\r"); // Bring cursor to start.

        // Write the output.
        print!("{}", output);

        // Write another newline so that the output is not cleared by the bar.
        eprintln!();
        eprintln!();
    } else {
        print!("{}", output);
    }

    // Print another line between solutions.
    println!();
}

pub trait Writer {
    fn write(&mut self, s: &str);
}

pub type ProgressWriter = Box<dyn Writer>;

pub fn get_writer(output_last: bool) -> ProgressWriter {
    let mut writer: ProgressWriter = Box::new(ProgressBarWriter {});
    if output_last {
        writer = Box::new(LastItemWriter::new(writer));
    }
    writer
}

struct ProgressBarWriter {}
impl Writer for ProgressBarWriter {
    fn write(&mut self, s: &str) {
        print_above_progress_bar(s);
    }
}

pub struct EmptyWriter {}
impl Writer for EmptyWriter {
    fn write(&mut self, _s: &str) {}
}

lazy_static! {
    static ref LAST_ITEM: Mutex<String> = Mutex::new(String::new());
}

pub fn set_ctrlc_handler() {
    LastItemWriter::set_ctrlc_handler();
}

struct LastItemWriter {
    last_item: String,
    wrapped_writer: ProgressWriter,
}

impl LastItemWriter {
    pub fn new(wrapped_writer: ProgressWriter) -> LastItemWriter {
        LastItemWriter {
            wrapped_writer,
            last_item: String::new(),
        }
    }

    pub fn set_ctrlc_handler() {
        ctrlc::set_handler(|| {
            // Print a new line so that we aren't on the same line as the '^C'
            eprintln!();
            // Write the last line.
            print!("{}", *LAST_ITEM.lock().unwrap());
            // Exit the process.
            std::process::exit(1);
        })
        .expect("Error setting Ctrl-C handler");
    }
}
impl Writer for LastItemWriter {
    fn write(&mut self, s: &str) {
        if !s.chars().all(|c| c == '\n') {
            // Only look at non-empty lines.
            self.last_item = s.to_string();
            *LAST_ITEM.lock().unwrap() = s.to_string();
        }
    }
}
impl Drop for LastItemWriter {
    fn drop(&mut self) {
        self.wrapped_writer.write(&self.last_item);
        *LAST_ITEM.lock().unwrap() = String::new();
    }
}
