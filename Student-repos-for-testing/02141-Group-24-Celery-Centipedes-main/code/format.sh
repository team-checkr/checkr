#!/bin/sh
cd "$(dirname "$0")"
dotnet tool restore
dotnet fantomas -r src/
