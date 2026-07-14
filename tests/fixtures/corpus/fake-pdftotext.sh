#!/bin/sh
if [ "$1" = "-v" ]; then
  echo "pdftotext version 26.04.0" >&2
  exit 0
fi
mode="layout"
output=""
for argument in "$@"; do
  [ "$argument" = "-bbox-layout" ] && mode="bbox"
  output="$argument"
done
if [ "$mode" = "bbox" ]; then
  printf '%s' '<html><body><doc><page width="595" height="842"><flow><block xMin="10" yMin="10" xMax="55" yMax="30"><line xMin="10" yMin="10" xMax="55" yMax="30"><word xMin="10" yMin="10" xMax="40" yMax="30">PAGE</word><word xMin="45" yMin="10" xMax="55" yMax="30">7</word></line></block></flow></page><page width="595" height="842"><flow><block xMin="10" yMin="10" xMax="55" yMax="30"><line xMin="10" yMin="10" xMax="55" yMax="30"><word xMin="10" yMin="10" xMax="40" yMax="30">PAGE</word><word xMin="45" yMin="10" xMax="55" yMax="30">8</word></line></block></flow></page></doc></body></html>' > "$output"
else
  printf 'PAGE 7\nBody\n\014PAGE 8\nBody\n' > "$output"
fi
