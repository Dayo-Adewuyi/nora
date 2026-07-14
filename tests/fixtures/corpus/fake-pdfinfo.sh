#!/bin/sh
if [ "$1" = "-v" ]; then
  echo "pdfinfo version 26.04.0" >&2
else
  printf "Pages: 2\nEncrypted: no\nFile size: 12 bytes\nPDF version: 1.7\n"
fi
