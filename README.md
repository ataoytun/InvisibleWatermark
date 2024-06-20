# Invisible Watermark

![UI](https://github.com/ataoytun/InvisibleWatermark/blob/main/readme.jpg?raw=true)


## Overview
The Invisible Watermark application is a Windows Forms-based tool created as a fun proof of concept to show the practicality of embedding invisible watermarks in images. This tool is especially useful for AI-generated images from platforms such as DALL-E, MidJourney and so on... Allowing creators to discreetly assert their ownership.

## Features
- **Bulk Watermarking:** Process an entire folder of images, applying watermarks to all applicable files.
- **Watermark Extraction:** Retrieve embedded watermarks from images to verify authenticity or ownership.
- **Interactive UI:** A user-friendly interface with asynchronous operations to ensure the application remains responsive.

## Prerequisites
- .NET Framework 8.0 or higher
- Windows OS (Windows 10 recommended)

## Download
To use the Invisible Watermark tool, you can download the latest version of the source code directly from GitHub:
1. Visit the project's GitHub page: [GitHub Repository Link](https://github.com/ataoytun/InvisibleWatermark)
2. Click on the **Code** button and then select **Download ZIP** from the dropdown menu.
3. Extract the downloaded ZIP file and navigate into the extracted folder.

## Usage
1. **Start the Application:**
   After building, run the application through Visual Studio or execute the built executable from the `bin` directory created during the build process.

2. **Set a Watermark Text:**
   - Choose 'Change Watermark' from the menu to set the text of the invisible watermark.

3. **Process Multiple Images:**
   - Use the 'Select Folder' option to choose a directory of images for bulk watermarking.
   - Specify the output folder where the watermarked images will be saved.
  
4. **Load an Image:**
   - Use the 'Load Single Image' option from the menu to select and load an image.
   - The loaded image will be displayed in the main window.

5. **Extract a Watermark:**
   - With an image loaded, select 'Extract Watermark' from the menu to display any existing watermark on the image.

## Contributing
Contributions are welcome! Please feel free to fork the repository and submit pull requests. You can also open issues to report bugs or suggest new features.

1. **Fork the Repository:** Create your own fork and work on your enhancements or fixes.
2. **Create Pull Requests:** Submit pull requests for your changes to be reviewed and merged into the main project.
3. **Report Issues:** Use the GitHub issues tracker to report bugs or suggest enhancements.
