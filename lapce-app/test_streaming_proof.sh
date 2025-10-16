#!/bin/bash
cd /home/verma/lapce/lapce-app
export GEMINI_API_KEY='AIzaSyCr-9z7jQ-co7IvuvxCWvhvitxMOINIXcU'
export GEMINI_UI_TEST=1

cargo build --example gemini_chatbot --quiet 2>/dev/null
timeout 120 ./target/debug/examples/gemini_chatbot 2>&1 | grep -E "^\[SSE @|^\[UI @"
