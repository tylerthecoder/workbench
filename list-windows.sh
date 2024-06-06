#!/bin/bash

window_ids=$(xdotool search "")

# Iterate through each window ID
for window_id in $window_ids; do
    # Get the window name
    window_name=$(xdotool getwindowname $window_id)
    window_classname=$(xdotool getwindowclassname $window_id)
    echo "Window ID: $window_id"
    echo "Window Name: $window_name"
    echo "Window Class Name: $window_classname"
    echo "--------------------------------"
done

