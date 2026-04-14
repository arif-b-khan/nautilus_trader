#!/bin/bash

awk -F'"' '
	/channel[[:space:]]*=/ {
		gsub(/[[:space:]]/, "", $2)
		print $2
		exit
	}
	/version[[:space:]]*=/ {
		gsub(/[[:space:]]/, "", $2)
		print $2
		exit
	}
' rust-toolchain.toml
