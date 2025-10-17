//! Tool Description Generators
//!
//! 1:1 Translation from Codex `src/core/prompts/tools/*.ts`
//!
//! Each function generates the markdown description for a specific tool

use std::path::Path;

/// Arguments passed to tool description generators
pub struct ToolDescriptionArgs<'a> {
    pub workspace: &'a Path,
    pub supports_browser: bool,
    pub max_concurrent_file_reads: usize,
    pub partial_reads_enabled: bool,
}

/// Generate read_file tool description
///
/// Translation from read-file.ts (lines 8-95)
pub fn read_file_description(args: &ToolDescriptionArgs) -> String {
    let is_multiple_reads_enabled = args.max_concurrent_file_reads > 1;
    let cwd = args.workspace.display();
    
    let multiple_reads_note = if is_multiple_reads_enabled {
        format!("**IMPORTANT: You can read a maximum of {} files in a single request.** If you need to read more files, use multiple sequential read_file requests.", args.max_concurrent_file_reads)
    } else {
        "**IMPORTANT: Multiple file reads are currently disabled. You can only read one file at a time.**".to_string()
    };
    
    let partial_reads_note = if args.partial_reads_enabled {
        "By specifying line ranges, you can efficiently read specific portions of large files without loading the entire file into memory."
    } else {
        ""
    };
    
    let line_range_param = if args.partial_reads_enabled {
        "  - line_range: (optional) One or more line range elements in format \"start-end\" (1-based, inclusive)"
    } else {
        ""
    };
    
    let line_range_usage = if args.partial_reads_enabled {
        "    <line_range>start-end</line_range>"
    } else {
        ""
    };
    
    let multiple_files_example = if is_multiple_reads_enabled {
        format!(r#"
2. Reading multiple files (within the {}-file limit):
<read_file>
<args>
  <file>
    <path>src/app.ts</path>
    {}
  </file>
  <file>
    <path>src/utils.ts</path>
    {}
  </file>
</args>
</read_file>"#, 
        args.max_concurrent_file_reads,
        if args.partial_reads_enabled { "<line_range>1-50</line_range>\n    <line_range>100-150</line_range>" } else { "" },
        if args.partial_reads_enabled { "<line_range>10-20</line_range>" } else { "" }
        )
    } else {
        String::new()
    };
    
    let example_number = if is_multiple_reads_enabled { "3" } else { "2" };
    
    let efficient_reading = if is_multiple_reads_enabled {
        format!("You MUST read all related files and implementations together in a single operation (up to {} files at once)", args.max_concurrent_file_reads)
    } else {
        "You MUST read files one at a time, as multiple file reads are currently disabled".to_string()
    };
    
    let partial_reading_strategy = if args.partial_reads_enabled {
        r#"- You MUST use line ranges to read specific portions of large files, rather than reading entire files when not needed
- You MUST combine adjacent line ranges (<10 lines apart)
- You MUST use multiple ranges for content separated by >10 lines
- You MUST include sufficient line context for planned modifications while keeping ranges minimal
"#
    } else {
        ""
    };
    
    format!(r#"## read_file
Description: Request to read the contents of {}. The tool outputs line-numbered content (e.g. "1 | const x = 1") for easy reference when creating diffs or discussing code.{} Supports text extraction from PDF and DOCX files, but may not handle other binary files properly.

{}

{}
Parameters:
- args: Contains one or more file elements, where each file contains:
  - path: (required) File path (relative to workspace directory {})
  {}

Usage:
<read_file>
<args>
  <file>
    <path>path/to/file</path>
    {}
  </file>
</args>
</read_file>

Examples:

1. Reading a single file:
<read_file>
<args>
  <file>
    <path>src/app.ts</path>
    {}
  </file>
</args>
</read_file>
{}

{}. Reading an entire file:
<read_file>
<args>
  <file>
    <path>config.json</path>
  </file>
</args>
</read_file>

IMPORTANT: You MUST use this Efficient Reading Strategy:
- {}
- You MUST obtain all necessary context before proceeding with changes
{}
{}"#,
        if is_multiple_reads_enabled { "one or more files" } else { "a file" },
        if args.partial_reads_enabled { " Use line ranges to efficiently read specific portions of large files." } else { "" },
        multiple_reads_note,
        partial_reads_note,
        cwd,
        line_range_param,
        line_range_usage,
        line_range_usage,
        multiple_files_example,
        example_number,
        efficient_reading,
        partial_reading_strategy,
        if is_multiple_reads_enabled { format!("- When you need to read more than {} files, prioritize the most critical files first, then use subsequent read_file requests for additional files", args.max_concurrent_file_reads) } else { String::new() }
    )
}

/// Generate write_to_file tool description
///
/// Translation from write-to-file.ts (lines 3-40)
pub fn write_to_file_description(workspace: &Path) -> String {
    let cwd = workspace.display();
    let example_json = "{
  \"apiEndpoint\": \"https://api.example.com\",
  \"theme\": {
    \"primaryColor\": \"#007bff\",
    \"secondaryColor\": \"#6c757d\",
    \"fontFamily\": \"Arial, sans-serif\"
  },
  \"features\": {
    \"darkMode\": true,
    \"notifications\": true,
    \"analytics\": false
  },
  \"version\": \"1.0.0\"
}";
    
    format!(r#"## write_to_file
Description: Request to write content to a file. This tool is primarily used for **creating new files** or for scenarios where a **complete rewrite of an existing file is intentionally required**. If the file exists, it will be overwritten. If it doesn't exist, it will be created. This tool will automatically create any directories needed to write the file.
Parameters:
- path: (required) The path of the file to write to (relative to the current workspace directory {})
- content: (required) The content to write to the file. When performing a full rewrite of an existing file or creating a new one, ALWAYS provide the COMPLETE intended content of the file, without any truncation or omissions. You MUST include ALL parts of the file, even if they haven't been modified. Do NOT include the line numbers in the content though, just the actual content of the file.
- line_count: (required) The number of lines in the file. Make sure to compute this based on the actual content of the file, not the number of lines in the content you're providing.
Usage:
<write_to_file>
<path>File path here</path>
<content>
Your file content here
</content>
<line_count>total number of lines in the file, including empty lines</line_count>
</write_to_file>

Example: Requesting to write to frontend-config.json
<write_to_file>
<path>frontend-config.json</path>
<content>
{}
</content>
<line_count>14</line_count>
</write_to_file>"#,
        cwd, example_json
    )
}

/// Generate execute_command tool description
///
/// Translation from execute-command.ts (lines 3-25)
pub fn execute_command_description(workspace: &Path) -> String {
    format!(r#"## execute_command
Description: Request to execute a CLI command on the system. Use this when you need to perform system operations or run specific commands to accomplish any step in the user's task. You must tailor your command to the user's system and provide a clear explanation of what the command does. For command chaining, use the appropriate chaining syntax for the user's shell. Prefer to execute complex CLI commands over creating executable scripts, as they are more flexible and easier to run. Prefer relative commands and paths that avoid location sensitivity for terminal consistency, e.g: `touch ./testdata/example.file`, `dir ./examples/model1/data/yaml`, or `go test ./cmd/front --config ./cmd/front/config.yml`. If directed by the user, you may open a terminal in a different directory by using the `cwd` parameter.
Parameters:
- command: (required) The CLI command to execute. This should be valid for the current operating system. Ensure the command is properly formatted and does not contain any harmful instructions.
- cwd: (optional) The working directory to execute the command in (default: {})
Usage:
<execute_command>
<command>Your command here</command>
<cwd>Working directory path (optional)</cwd>
</execute_command>

Example: Requesting to execute npm run dev
<execute_command>
<command>npm run dev</command>
</execute_command>

Example: Requesting to execute ls in a specific directory if directed
<execute_command>
<command>ls -la</command>
<cwd>/home/user/projects</cwd>
</execute_command>"#,
        workspace.display()
    )
}

/// Generate list_files tool description
///
/// Translation from list-files.ts (lines 3-20)
pub fn list_files_description(workspace: &Path) -> String {
    format!(r#"## list_files
Description: Request to list files and directories within the specified directory. If recursive is true, it will list all files and directories recursively. If recursive is false or not provided, it will only list the top-level contents. Do not use this tool to confirm the existence of files you may have created, as the user will let you know if the files were created successfully or not.
Parameters:
- path: (required) The path of the directory to list contents for (relative to the current workspace directory {})
- recursive: (optional) Whether to list files recursively. Use true for recursive listing, false or omit for top-level only.
Usage:
<list_files>
<path>Directory path here</path>
<recursive>true or false (optional)</recursive>
</list_files>

Example: Requesting to list all files in the current directory
<list_files>
<path>.</path>
<recursive>false</recursive>
</list_files>"#,
        workspace.display()
    )
}

/// Generate search_files tool description
///
/// Translation from search-files.ts (lines 3-23)
pub fn search_files_description(workspace: &Path) -> String {
    format!(r#"## search_files
Description: Request to perform a regex search across files in a specified directory, providing context-rich results. This tool searches for patterns or specific content across multiple files, displaying each match with encapsulating context.
Parameters:
- path: (required) The path of the directory to search in (relative to the current workspace directory {}). This directory will be recursively searched.
- regex: (required) The regular expression pattern to search for. Uses Rust regex syntax.
- file_pattern: (optional) Glob pattern to filter files (e.g., '*.ts' for TypeScript files). If not provided, it will search all files (*).
Usage:
<search_files>
<path>Directory path here</path>
<regex>Your regex pattern here</regex>
<file_pattern>file pattern here (optional)</file_pattern>
</search_files>

Example: Requesting to search for all .ts files in the current directory
<search_files>
<path>.</path>
<regex>.*</regex>
<file_pattern>*.ts</file_pattern>
</search_files>"#,
        workspace.display()
    )
}

/// Generate insert_content tool description
///
/// Translation from insert-content.ts (lines 3-33)
pub fn insert_content_description(workspace: &Path) -> String {
    format!(r#"## insert_content
Description: Use this tool specifically for adding new lines of content into a file without modifying existing content. Specify the line number to insert before, or use line 0 to append to the end. Ideal for adding imports, functions, configuration blocks, log entries, or any multi-line text block.

Parameters:
- path: (required) File path relative to workspace directory {}
- line: (required) Line number where content will be inserted (1-based)
	      Use 0 to append at end of file
	      Use any positive number to insert before that line
- content: (required) The content to insert at the specified line

Example for inserting imports at start of file:
<insert_content>
<path>src/utils.ts</path>
<line>1</line>
<content>
// Add imports at start of file
import {{ sum }} from './math';
</content>
</insert_content>

Example for appending to the end of file:
<insert_content>
<path>src/utils.ts</path>
<line>0</line>
<content>
// This is the end of the file
</content>
</insert_content>
"#,
        workspace.display()
    )
}

/// Generate search_and_replace tool description
///
/// Translation from search-and-replace.ts (lines 3-39)
pub fn search_and_replace_description(workspace: &Path) -> String {
    format!(r#"## search_and_replace
Description: Use this tool to find and replace specific text strings or patterns (using regex) within a file. It's suitable for targeted replacements across multiple locations within the file. Supports literal text and regex patterns, case sensitivity options, and optional line ranges. Shows a diff preview before applying changes.

Required Parameters:
- path: The path of the file to modify (relative to the current workspace directory {})
- search: The text or pattern to search for
- replace: The text to replace matches with

Optional Parameters:
- start_line: Starting line number for restricted replacement (1-based)
- end_line: Ending line number for restricted replacement (1-based)
- use_regex: Set to "true" to treat search as a regex pattern (default: false)
- ignore_case: Set to "true" to ignore case when matching (default: false)

Notes:
- When use_regex is true, the search parameter is treated as a regular expression pattern
- When ignore_case is true, the search is case-insensitive regardless of regex mode

Examples:

1. Simple text replacement:
<search_and_replace>
<path>example.ts</path>
<search>oldText</search>
<replace>newText</replace>
</search_and_replace>

2. Case-insensitive regex pattern:
<search_and_replace>
<path>example.ts</path>
<search>old\w+</search>
<replace>new$&</replace>
<use_regex>true</use_regex>
<ignore_case>true</ignore_case>
</search_and_replace>"#,
        workspace.display()
    )
}

/// Generate ask_followup_question tool description
///
/// Translation from ask-followup-question.ts (lines 1-27)
pub fn ask_followup_question_description() -> String {
    r#"## ask_followup_question
Description: Ask the user a question to gather additional information needed to complete the task. Use when you need clarification or more details to proceed effectively.

Parameters:
- question: (required) A clear, specific question addressing the information needed
- follow_up: (optional) A list of 2-4 suggested answers, each in its own <suggest> tag. Suggestions must be complete, actionable answers without placeholders. Optionally include mode attribute to switch modes (code/architect/etc.)

Usage:
<ask_followup_question>
<question>Your question here</question>
<follow_up>
<suggest>First suggestion</suggest>
<suggest mode="code">Action with mode switch</suggest>
</follow_up>
</ask_followup_question>

Example:
<ask_followup_question>
<question>What is the path to the frontend-config.json file?</question>
<follow_up>
<suggest>./src/frontend-config.json</suggest>
<suggest>./config/frontend-config.json</suggest>
<suggest>./frontend-config.json</suggest>
</follow_up>
</ask_followup_question>"#.to_string()
}

/// Generate attempt_completion tool description
///
/// Translation from attempt-completion.ts (lines 3-22)
pub fn attempt_completion_description() -> String {
    r#"## attempt_completion
Description: After each tool use, the user will respond with the result of that tool use, i.e. if it succeeded or failed, along with any reasons for failure. Once you've received the results of tool uses and can confirm that the task is complete, use this tool to present the result of your work to the user. The user may respond with feedback if they are not satisfied with the result, which you can use to make improvements and try again.
IMPORTANT NOTE: This tool CANNOT be used until you've confirmed from the user that any previous tool uses were successful. Failure to do so will result in code corruption and system failure. Before using this tool, you must confirm that you've received successful results from the user for any previous tool uses. If not, then DO NOT use this tool.
Parameters:
- result: (required) The result of the task. Formulate this result in a way that is final and does not require further input from the user. Don't end your result with questions or offers for further assistance.
Usage:
<attempt_completion>
<result>
Your final result description here
</result>
</attempt_completion>

Example: Requesting to attempt completion with a result
<attempt_completion>
<result>
I've updated the CSS
</result>
</attempt_completion>"#.to_string()
}

/// Generate list_code_definition_names tool description
///
/// Translation from list-code-definition-names.ts (lines 3-24)
pub fn list_code_definition_names_description(workspace: &Path) -> String {
    format!(r#"## list_code_definition_names
Description: Request to list definition names (classes, functions, methods, etc.) from source code. This tool can analyze either a single file or all files at the top level of a specified directory. It provides insights into the codebase structure and important constructs, encapsulating high-level concepts and relationships that are crucial for understanding the overall architecture.
Parameters:
- path: (required) The path of the file or directory (relative to the current working directory {}) to analyze. When given a directory, it lists definitions from all top-level source files.
Usage:
<list_code_definition_names>
<path>Directory path here</path>
</list_code_definition_names>

Examples:

1. List definitions from a specific file:
<list_code_definition_names>
<path>src/main.ts</path>
</list_code_definition_names>

2. List definitions from all files in a directory:
<list_code_definition_names>
<path>src/</path>
</list_code_definition_names>"#,
        workspace.display()
    )
}

/// Generate browser_action tool description
///
/// Translation from browser-action.ts (lines 3-59)
pub fn browser_action_description(workspace: &Path, browser_viewport_size: &str) -> String {
    format!(r#"## browser_action
Description: Request to interact with a Puppeteer-controlled browser. Every action, except `close`, will be responded to with a screenshot of the browser's current state, along with any new console logs. You may only perform one browser action per message, and wait for the user's response including a screenshot and logs to determine the next action.
- The sequence of actions **must always start with** launching the browser at a URL, and **must always end with** closing the browser. If you need to visit a new URL that is not possible to navigate to from the current webpage, you must first close the browser, then launch again at the new URL.
- While the browser is active, only the `browser_action` tool can be used. No other tools should be called during this time. You may proceed to use other tools only after closing the browser. For example if you run into an error and need to fix a file, you must close the browser, then use other tools to make the necessary changes, then re-launch the browser to verify the result.
- The browser window has a resolution of **{}** pixels. When performing any click actions, ensure the coordinates are within this resolution range.
- Before clicking on any elements such as icons, links, or buttons, you must consult the provided screenshot of the page to determine the coordinates of the element. The click should be targeted at the **center of the element**, not on its edges.
Parameters:
- action: (required) The action to perform. The available actions are:
    * launch: Launch a new Puppeteer-controlled browser instance at the specified URL. This **must always be the first action**.
        - Use with the `url` parameter to provide the URL.
        - Ensure the URL is valid and includes the appropriate protocol (e.g. http://localhost:3000/page, file:///path/to/file.html, etc.)
    * hover: Move the cursor to a specific x,y coordinate.
        - Use with the `coordinate` parameter to specify the location.
        - Always move to the center of an element (icon, button, link, etc.) based on coordinates derived from a screenshot.
    * click: Click at a specific x,y coordinate.
        - Use with the `coordinate` parameter to specify the location.
        - Always click in the center of an element (icon, button, link, etc.) based on coordinates derived from a screenshot.
    * type: Type a string of text on the keyboard. You might use this after clicking on a text field to input text.
        - Use with the `text` parameter to provide the string to type.
    * resize: Resize the viewport to a specific w,h size.
        - Use with the `size` parameter to specify the new size.
    * scroll_down: Scroll down the page by one page height.
    * scroll_up: Scroll up the page by one page height.
    * close: Close the Puppeteer-controlled browser instance. This **must always be the final browser action**.
        - Example: `<action>close</action>`
- url: (optional) Use this for providing the URL for the `launch` action.
    * Example: <url>https://example.com</url>
- coordinate: (optional) The X and Y coordinates for the `click` and `hover` actions. Coordinates should be within the **{}** resolution.
    * Example: <coordinate>450,300</coordinate>
- size: (optional) The width and height for the `resize` action.
    * Example: <size>1280,720</size>
- text: (optional) Use this for providing the text for the `type` action.
    * Example: <text>Hello, world!</text>
Usage:
<browser_action>
<action>Action to perform (e.g., launch, click, type, scroll_down, scroll_up, close)</action>
<url>URL to launch the browser at (optional)</url>
<coordinate>x,y coordinates (optional)</coordinate>
<text>Text to type (optional)</text>
</browser_action>

Example: Requesting to launch a browser at https://example.com
<browser_action>
<action>launch</action>
<url>https://example.com</url>
</browser_action>

Example: Requesting to click on the element at coordinates 450,300
<browser_action>
<action>click</action>
<coordinate>450,300</coordinate>
</browser_action>"#,
        browser_viewport_size, browser_viewport_size
    )
}

/// Generate codebase_search tool description
///
/// Translation from codebase-search.ts (lines 3-23)
pub fn codebase_search_description(workspace: &Path) -> String {
    format!(r#"## codebase_search
Description: Find files most relevant to the search query using semantic search. Searches based on meaning rather than exact text matches. By default searches entire workspace. Reuse the user's exact wording unless there's a clear reason not to - their phrasing often helps semantic search. Queries MUST be in English (translate if needed).

Parameters:
- query: (required) The search query. Reuse the user's exact wording/question format unless there's a clear reason not to.
- path: (optional) Limit search to specific subdirectory (relative to the current workspace directory {}). Leave empty for entire workspace.

Usage:
<codebase_search>
<query>Your natural language query here</query>
<path>Optional subdirectory path</path>
</codebase_search>

Example:
<codebase_search>
<query>User login and password hashing</query>
<path>src/auth</path>
</codebase_search>
"#,
        workspace.display()
    )
}

/// Generate switch_mode tool description
///
/// Translation from switch-mode.ts (lines 1-18)
pub fn switch_mode_description() -> String {
    r#"## switch_mode
Description: Request to switch to a different mode. This tool allows modes to request switching to another mode when needed, such as switching to Code mode to make code changes. The user must approve the mode switch.
Parameters:
- mode_slug: (required) The slug of the mode to switch to (e.g., "code", "ask", "architect")
- reason: (optional) The reason for switching modes
Usage:
<switch_mode>
<mode_slug>Mode slug here</mode_slug>
<reason>Reason for switching here</reason>
</switch_mode>

Example: Requesting to switch to code mode
<switch_mode>
<mode_slug>code</mode_slug>
<reason>Need to make code changes</reason>
</switch_mode>"#.to_string()
}

/// Generate new_task tool description
///
/// Translation from new-task.ts (lines 6-67)
pub fn new_task_description(todos_required: bool) -> String {
    if todos_required {
        r#"## new_task
Description: This will let you create a new task instance in the chosen mode using your provided message and initial todo list.

Parameters:
- mode: (required) The slug of the mode to start the new task in (e.g., "code", "debug", "architect").
- message: (required) The initial user message or instructions for this new task.
- todos: (required) The initial todo list in markdown checklist format for the new task.

Usage:
<new_task>
<mode>your-mode-slug-here</mode>
<message>Your initial instructions here</message>
<todos>
[ ] First task to complete
[ ] Second task to complete
[ ] Third task to complete
</todos>
</new_task>

Example:
<new_task>
<mode>code</mode>
<message>Implement user authentication</message>
<todos>
[ ] Set up auth middleware
[ ] Create login endpoint
[ ] Add session management
[ ] Write tests
</todos>
</new_task>

"#.to_string()
    } else {
        r#"## new_task
Description: This will let you create a new task instance in the chosen mode using your provided message.

Parameters:
- mode: (required) The slug of the mode to start the new task in (e.g., "code", "debug", "architect").
- message: (required) The initial user message or instructions for this new task.

Usage:
<new_task>
<mode>your-mode-slug-here</mode>
<message>Your initial instructions here</message>
</new_task>

Example:
<new_task>
<mode>code</mode>
<message>Implement a new feature for the application</message>
</new_task>
"#.to_string()
    }
}

/// Generate update_todo_list tool description
///
/// Translation from update-todo-list.ts (lines 6-75)
pub fn update_todo_list_description() -> String {
    r#"## update_todo_list

**Description:**
Replace the entire TODO list with an updated checklist reflecting the current state. Always provide the full list; the system will overwrite the previous one. This tool is designed for step-by-step task tracking, allowing you to confirm completion of each step before updating, update multiple task statuses at once (e.g., mark one as completed and start the next), and dynamically add new todos discovered during long or complex tasks.

**Checklist Format:**
- Use a single-level markdown checklist (no nesting or subtasks).
- List todos in the intended execution order.
- Status options:
	 - [ ] Task description (pending)
	 - [x] Task description (completed)
	 - [-] Task description (in progress)

**Status Rules:**
- [ ] = pending (not started)
- [x] = completed (fully finished, no unresolved issues)
- [-] = in_progress (currently being worked on)

**Core Principles:**
- Before updating, always confirm which todos have been completed since the last update.
- You may update multiple statuses in a single update (e.g., mark the previous as completed and the next as in progress).
- When a new actionable item is discovered during a long or complex task, add it to the todo list immediately.
- Do not remove any unfinished todos unless explicitly instructed.
- Always retain all unfinished tasks, updating their status as needed.
- Only mark a task as completed when it is fully accomplished (no partials, no unresolved dependencies).
- If a task is blocked, keep it as in_progress and add a new todo describing what needs to be resolved.
- Remove tasks only if they are no longer relevant or if the user requests deletion.

**Usage Example:**
<update_todo_list>
<todos>
[x] Analyze requirements
[x] Design architecture
[-] Implement core logic
[ ] Write tests
[ ] Update documentation
</todos>
</update_todo_list>

*After completing "Implement core logic" and starting "Write tests":*
<update_todo_list>
<todos>
[x] Analyze requirements
[x] Design architecture
[x] Implement core logic
[-] Write tests
[ ] Update documentation
[ ] Add performance benchmarks
</todos>
</update_todo_list>

**When to Use:**
- The task is complicated or involves multiple steps or requires ongoing tracking.
- You need to update the status of several todos at once.
- New actionable items are discovered during task execution.
- The user requests a todo list or provides multiple tasks.
- The task is complex and benefits from clear, stepwise progress tracking.

**When NOT to Use:**
- There is only a single, trivial task.
- The task can be completed in one or two simple steps.
- The request is purely conversational or informational.

**Task Management Guidelines:**
- Mark task as completed immediately after all work of the current task is done.
- Start the next task by marking it as in_progress.
- Add new todos as soon as they are identified.
- Use clear, descriptive task names.
"#.to_string()
}
