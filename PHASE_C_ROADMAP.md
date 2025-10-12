# Phase C UI Translation - Implementation Roadmap

## 🎯 Quick Start Guide

**Current Status:** 20% complete (2,606 lines)  
**Target:** 100% complete (~13,000 lines)  
**Time to MVP:** 2 weeks  
**Time to Complete:** 10 weeks  

---

## 📅 2-Week MVP Sprint (Critical Features Only)

### Week 1: Foundation
**Goal:** Build UI primitives and basic infrastructure

#### Days 1-2: Core Primitives
- [ ] Popover component (4h)
- [ ] Dialog/Modal component (3h)
- [ ] Dropdown menu (3h)
- [ ] Tooltip component (2h)
- [ ] Icon system (4h)

#### Days 3-4: Form Components
- [ ] Input component (2h)
- [ ] Textarea component (2h)
- [ ] Checkbox component (2h)
- [ ] Switch component (2h)
- [ ] Select component (3h)
- [ ] Button enhancements (2h)

#### Day 5: Tabs & Layout
- [ ] Tab component (2h)
- [ ] Badge enhancements (1h)
- [ ] Test all primitives (3h)

---

### Week 2: Critical Features
**Goal:** Make the chat actually usable

#### Days 6-7: Model & Settings
- [ ] Model selector dropdown (4h)
- [ ] Basic settings panel (8h)
  - API key input
  - Provider selection
  - Model configuration

#### Days 8-9: File Handling
- [ ] File upload button + dialog (3h)
- [ ] Image upload + preview (4h)
- [ ] Thumbnails component (2h)
- [ ] Multiline input (2h)

#### Day 10: Task Management
- [ ] Task header component (5h)
  - Token/cost display
  - Context progress
  - Task actions
- [ ] History button + preview (3h)

**MVP Complete! ✅ User can:**
- Configure API keys
- Select models
- Upload files/images
- See task costs
- View history

---

## 📅 10-Week Complete Implementation

### Weeks 1-2: MVP (Above) ✅

### Weeks 3-4: High Priority Features
- Mode selector with search
- Auto-approve menu system
- Context management UI
- Todo list display
- Command execution display
- Queued messages
- Message enhancements

### Weeks 5-6: Settings Deep Dive
- Terminal settings
- Context management settings
- Prompts settings
- Auto-approve settings (detailed)
- Notification settings
- Browser settings
- Display settings

### Weeks 7-8: Advanced Features
- MCP integration UI
- History panel (full)
- Modes management
- Task timeline
- Search results display
- Warnings & notifications

### Weeks 9-10: Polish & Testing
- Cloud features UI
- Marketplace (if needed)
- Component tests
- Integration tests
- Manual QA
- Bug fixes

---

## 🏗️ Build Order (Dependency Graph)

```
UI Primitives (Week 1)
    ↓
Basic Components (Week 1-2)
    ↓
Core Features (Week 2)
    ├─→ Settings System → Advanced Settings (Weeks 5-6)
    ├─→ Mode System → Mode Management (Weeks 7-8)
    ├─→ History System → Full History (Weeks 7-8)
    └─→ Upload System → Advanced Context (Weeks 3-4)
```

---

## 📊 Progress Tracking

### Components by Category

**UI Primitives:** 0/16 (0%)
- Popover, Dialog, Dropdown, Tabs, Tooltip
- Input, Checkbox, Switch, Select, Textarea
- Button, Badge, Icon, Thumbnails, Mention, IconButton

**Chat Core:** 4/10 (40%)
- ✅ ChatView (basic)
- ✅ ChatRow
- ✅ ChatTextArea (basic)
- ✅ WelcomeScreen (basic)
- ❌ TaskHeader
- ❌ ModelSelector
- ❌ HistoryPreview
- ❌ FileUpload
- ❌ ImageUpload
- ❌ MentionSupport

**Settings:** 0/18 (0%)
- All settings panels missing

**Tool Renderers:** 5/5 (100%) ✅
- ✅ File operations
- ✅ Diff operations
- ✅ Command operations
- ✅ Task operations
- ✅ MCP operations

**MCP Integration:** 0/4 (0%)
- MCP view, resources, tools, errors

**History:** 0/3 (0%)
- History view, task items, dialogs

**Modes:** 0/3 (0%)
- Mode selector, edit controls, modes view

**Notifications:** 0/7 (0%)
- Warnings, banners, notifications

**Cloud/Marketplace:** 0/7 (0%)
- Cloud features, marketplace, org management

**State/Hooks:** 0/15 (0%)
- State management, custom hooks, utils

---

## 🎯 Daily Task Breakdown (MVP - Week 1)

### Monday
**Morning (4h):**
- [ ] Setup UI component module structure
- [ ] Create base styling utilities
- [ ] Build Popover component with positioning

**Afternoon (4h):**
- [ ] Build Dialog/Modal component
- [ ] Add overlay & focus trap
- [ ] Test popover + dialog interaction

---

### Tuesday
**Morning (4h):**
- [ ] Build Dropdown menu component
- [ ] Add keyboard navigation
- [ ] Build Tooltip component

**Afternoon (4h):**
- [ ] Icon system setup
- [ ] Create icon component wrapper
- [ ] Add common icons (chevron, close, check, etc.)

---

### Wednesday
**Morning (4h):**
- [ ] Build Input component with variants
- [ ] Build Textarea component
- [ ] Add validation states

**Afternoon (4h):**
- [ ] Build Checkbox component
- [ ] Build Switch/Toggle component
- [ ] Style all form components

---

### Thursday
**Morning (4h):**
- [ ] Build Select component
- [ ] Add search to select
- [ ] Enhance Button component

**Afternoon (4h):**
- [ ] Build Tab component
- [ ] Add tab navigation
- [ ] Badge enhancements

---

### Friday
**Morning (4h):**
- [ ] Test all primitives together
- [ ] Fix styling issues
- [ ] Add keyboard navigation

**Afternoon (4h):**
- [ ] Document components
- [ ] Create component showcase
- [ ] Prepare for Week 2

---

## 🎯 Daily Task Breakdown (MVP - Week 2)

### Monday
**Morning (4h):**
- [ ] Build model selector UI
- [ ] Add model list
- [ ] Add model search

**Afternoon (4h):**
- [ ] Add model info display
- [ ] Test model selection
- [ ] Hook up to state

---

### Tuesday
**Morning (4h):**
- [ ] Start settings panel structure
- [ ] Add provider selector
- [ ] Add API key input

**Afternoon (4h):**
- [ ] Add model configuration
- [ ] Add save/cancel buttons
- [ ] Test settings flow

---

### Wednesday
**Morning (4h):**
- [ ] Build file upload button
- [ ] Add file picker dialog
- [ ] Add file type validation

**Afternoon (4h):**
- [ ] Build image upload
- [ ] Add image preview
- [ ] Build thumbnails component

---

### Thursday
**Morning (4h):**
- [ ] Add multiline input support
- [ ] Add auto-expand
- [ ] Add Shift+Enter handling

**Afternoon (4h):**
- [ ] Start task header component
- [ ] Add token count display
- [ ] Add cost display

---

### Friday
**Morning (4h):**
- [ ] Add context progress bar
- [ ] Add task actions menu
- [ ] Build history button

**Afternoon (4h):**
- [ ] Build history preview panel
- [ ] Test all MVP features
- [ ] Bug fixes & polish

---

## 🚀 Getting Started

### Step 1: Choose Your Approach
**Option A: MVP First (Recommended)**
- Focus on 2-week sprint
- Get usable chat ASAP
- Add features incrementally

**Option B: Full Implementation**
- Follow 10-week plan
- Build everything at once
- More complete but slower

### Step 2: Setup
```bash
cd /home/verma/lapce/lapce-app
# Create component directories
mkdir -p src/panel/ai_chat/ui/primitives
mkdir -p src/panel/ai_chat/components/settings
mkdir -p src/panel/ai_chat/components/mcp
mkdir -p src/panel/ai_chat/components/history
```

### Step 3: Start Building
```bash
# Day 1: Build first primitive
vim src/panel/ai_chat/ui/primitives/popover.rs
```

### Step 4: Test Continuously
```bash
# Rebuild and test
cargo build --package lapce-app --release
./target/release/lapce
```

---

## 📋 Checklist Format

Use this for daily tracking:

```markdown
## Day X Progress

### Completed ✅
- [x] Component A (2h actual vs 3h est)
- [x] Component B (4h actual vs 4h est)

### In Progress 🔄
- [ ] Component C (50% done)

### Blocked 🚫
- [ ] Component D (waiting for X)

### Tomorrow's Plan
- [ ] Component E
- [ ] Component F
```

---

## 🎖️ Milestones

### Milestone 1: Primitives Complete (End of Week 1)
**Success Criteria:**
- All 16 primitives built
- Working examples of each
- No known bugs

### Milestone 2: MVP Complete (End of Week 2)
**Success Criteria:**
- Can configure API
- Can select model
- Can upload files
- Can see costs
- Can view history

### Milestone 3: Feature Complete (End of Week 8)
**Success Criteria:**
- All settings work
- All modes work
- MCP integration works
- History fully functional

### Milestone 4: Polish Complete (End of Week 10)
**Success Criteria:**
- All tests pass
- No critical bugs
- Performance acceptable
- Documentation complete

---

## 💡 Tips for Success

1. **Build primitives first** - Everything depends on them
2. **Test as you go** - Don't accumulate bugs
3. **Keep it simple** - Match Codex behavior, don't over-engineer
4. **Reuse code** - DRY principle
5. **Document as you build** - Future you will thank you
6. **Ask for help** - When stuck, ask!
7. **Take breaks** - This is a marathon, not a sprint
8. **Celebrate wins** - Mark milestones!

---

## 📞 Need Help?

**Stuck on:**
- **Floem API?** Check Floem docs + examples
- **Layout issues?** Review existing Lapce components
- **State management?** Check Lapce panel state patterns
- **Component design?** Look at Codex source for reference

**Progress tracking:**
- Update `PHASE_C_COMPLETE_TODO.md` daily
- Mark items complete with ✅
- Note actual time taken
- Document blockers
