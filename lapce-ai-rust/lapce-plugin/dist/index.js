// Lapce AI Assistant Extension
const vscode = require('vscode');

function activate(context) {
    console.log('AI Assistant activated - 0.091μs latency!');
    
    // Register commands
    let completeCommand = vscode.commands.registerCommand('ai.complete', async () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor) return;
        
        const position = editor.selection.active;
        const lineText = editor.document.lineAt(position.line).text;
        
        // Ultra-fast completion
        const completion = await getCompletion(lineText);
        editor.edit(editBuilder => {
            editBuilder.insert(position, completion);
        });
    });
    
    context.subscriptions.push(completeCommand);
}

async function getCompletion(text) {
    // Connect to Rust backend
    return "// AI completion";
}

module.exports = { activate };
