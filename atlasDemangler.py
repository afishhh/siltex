import sys
from pathlib import Path
from PIL import Image

def extract_images(image_map_path, coords_file_path):
    # Determine the base directory (MainFolder)
    image_map_path = Path(image_map_path)
    coords_file_path = Path(coords_file_path)
    base_dir = image_map_path.parent.parent
    img_dir = base_dir / "img"
    
    # Open the large image map
    try:
        image_map = Image.open(image_map_path)
    except Exception as e:
        print(f"Error opening image: {e}")
        return
    
    # Read the coordinates file
    try:
        with coords_file_path.open("r", encoding="utf-8") as f:
            lines = f.readlines()
    except Exception as e:
        print(f"Error reading coordinates file: {e}")
        return
    
    for line in lines:
        parts = line.strip().split(maxsplit=5)  # Ensure correct splitting even with extra spaces
        if len(parts) != 5:
            print(f"Skipping invalid line: {line.strip()}")
            continue
        
        rel_path, x, y, width, height = parts[0], int(parts[1]), int(parts[2]), int(parts[3]), int(parts[4])
        output_path = base_dir / rel_path
        output_dir = output_path.parent
        
        # Create the target directory if it does not exist
        output_dir.mkdir(parents=True, exist_ok=True)
        
        # Crop the image and save it
        try:
            cropped_img = image_map.crop((x, y, x + width, y + height))
            cropped_img.save(output_path)
            print(f"Successfully saved: {output_path} (x:{x}, y:{y}, w:{width}, h:{height})")
        except Exception as e:
            print(f"Error saving {output_path}: {e}")

if __name__ == "__main__":
    if len(sys.argv) != 3:
        print("Usage: imageDemangler.py [imageMap.png] [imageCoords.txt]")
        sys.exit(1)
    
    extract_images(sys.argv[1], sys.argv[2])
