#!/usr/bin/env python3
import tkinter as tk
import sys
import re
import os

# --- Calibration Utility Class ---
class CalibrationUtility:
    def __init__(self, master):
        self.master = master
        master.title(f"X Position Calibration")
        master.geometry("800x200")
        master.attributes("-topmost", True) # Keep window on top

        self.left_bounds = []
        self.right_bounds = []
        self.current_list = self.left_bounds

        self.label = tk.Label(master, text=f"Move mouse to the leftmost edge of string 1 and press SPACE. Do this for all desired strings from left to right.\nWhen done, press ENTER. Then do the same for right edges. Press ENTER again to finish.")
        self.label.pack(pady=10)

        self.status_label = tk.Label(master, text=f"Collecting Left Bounds")
        self.status_label.pack(pady=5)

        self.current_x_label = tk.Label(master, text="Current X: 0.0000")
        self.current_x_label.pack(pady=5)

        master.bind("<KeyPress>", self.on_key_press)
        master.bind("<Motion>", self.on_mouse_move)

        self.mouse_x_percentage = 0.0

    def on_mouse_move(self, event):
        if self.master.winfo_width() > 0:
            self.mouse_x_percentage = event.x / self.master.winfo_width()
            self.current_x_label.config(text=f"Current X: {self.mouse_x_percentage:.4f}")

    def on_key_press(self, event):
        if event.keysym == "space":
            if self.current_list is self.left_bounds:
                # Append to left_bounds as we collect them from left to right
                self.current_list.append(self.mouse_x_percentage)
                self.status_label.config(text=f"Collecting Left Bounds ({len(self.left_bounds)})")
                print(f"Added left bound: {self.mouse_x_percentage:.17e}")

            elif self.current_list is self.right_bounds and len(self.right_bounds) < len(self.left_bounds):
                # Prepend to right_bounds so it will match up with the left.
                self.current_list.insert(0, self.mouse_x_percentage)
                self.status_label.config(text=f"Collecting Right Bounds ({len(self.right_bounds)}/{len(self.left_bounds)})")
                print(f"Added right bound: {self.mouse_x_percentage:.17e}")
                if len(self.right_bounds) == len(self.left_bounds):
                    print("Right bounds collection complete.")
                    self.status_label.config(text="Right bounds collection complete.")
                    self.calculate_and_update_rust_file()
                    self.master.quit()
        # Backspace for undo
        elif event.keysym == "BackSpace":
            if self.current_list and len(self.current_list) > 0:
                removed_value = self.current_list.pop()
                if self.current_list is self.left_bounds:
                    self.status_label.config(text=f"Collecting Left Bounds ({len(self.left_bounds)})")
                    print(f"Removed left bound: {removed_value:.17e}")
                else:
                    self.status_label.config(text=f"Collecting Right Bounds ({len(self.right_bounds)}/{len(self.left_bounds)})")
                    print(f"Removed right bound: {removed_value:.17e}")

        elif event.keysym == "Return": # Enter key
            if self.current_list is self.left_bounds:
                self.current_list = self.right_bounds
                self.status_label.config(text=f"Collecting Right Bounds ({len(self.right_bounds)}/{len(self.left_bounds)})")
                print("Switched to collecting right bounds.")

    def calculate_and_update_rust_file(self):
        if len(self.left_bounds) != len(self.right_bounds):
            print("Error: Mismatched number of bounds collected.")
            return

        final_x_positions = []
        for left, right in zip(self.left_bounds, self.right_bounds):
            avg_pos = (left + right) / 2.0
            final_x_positions.append(avg_pos)

        # Format for Rust array
        rust_array_string = f"const UNSCALED_RELATIVE_X_POSITIONS: &[f32] = &[\n"
        for pos in final_x_positions:
            rust_array_string += f"    {pos:.17e},\n"
        rust_array_string += "];"

        print(f"\n--- New UNSCALED_RELATIVE_X_POSITIONS for src/main.rs ---")
        print(rust_array_string)
        print("-----------------------------------------------------------")

        # Replace in src/main.rs
        try:
            script_dir = os.path.dirname(__file__)
            rust_file_path = os.path.join(script_dir, "src", "main.rs")

            with open(rust_file_path, "r") as f:
                original_content = f.read()

            # Regex to find the existing UNSCALED_RELATIVE_X_POSITIONS block
            # This pattern matches the start of the line, then the const declaration, then the array type with length, then the array content.
            array_pattern = r"^const UNSCALED_RELATIVE_X_POSITIONS: \&\[f32\] = \&\[(.*?)\];"

            if re.search(array_pattern, original_content, re.MULTILINE | re.DOTALL):
                original_content = re.sub(array_pattern, rust_array_string, original_content, count=1, flags=re.MULTILINE | re.DOTALL)
            else:
                print("Error: UNSCALED_RELATIVE_X_POSITIONS block not found in src/main.rs. Please update manually.")
                return # Stop if array not found

            with open(rust_file_path, "w") as f:
                f.write(original_content)
            print("src/main.rs updated successfully.")

        except FileNotFoundError:
            print(f"Error: src/main.rs not found at {rust_file_path}.")
        except Exception as e:
            print(f"Error updating src/main.rs: {e}")


if __name__ == "__main__":
    root = tk.Tk()
    app = CalibrationUtility(root)
    root.mainloop()
