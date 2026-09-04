#!/bin/bash

printf "%-15s %-25s %s\n" "ZONE" "TYPE" "TEMP(°C)"

for zone in /sys/class/thermal/thermal_zone*; do
	type=$(cat $zone/type 2>/dev/null)
	temp=$(cat $zone/temp 2>/dev/null)

	if [[ "$temp" =~ ^[0-9]+$ ]]; then
		temp_c=$(awk "BEGIN {printf \"%.1f\", $temp/1000}")
		printf "%-15s %-25s %s\n" "$(basename $zone)" "$type" "$temp_c"
	fi
done | sort -k3 -nr
