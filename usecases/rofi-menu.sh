workspace=$(bench list-workspaces | rofi -dmenu -i -p "Select workspace")

echo "Selected workspace: $workspace"

bench open "$workspace"


