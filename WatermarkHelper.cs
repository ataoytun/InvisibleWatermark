using System.Text;
using System.Drawing.Imaging;
using System.Runtime.InteropServices;
#pragma warning disable CA1416 // Ignores the warning for '....' which is only supported on Windows 6.1 and later.

namespace InvisibleWatermark
{
    public class WatermarkHelper
    {
        private const byte MinPrintableAscii = 32;
        private const byte MaxPrintableAscii = 126;
        private const int BitsPerByte = 8;
        private const int LeastSignificantBitMask = 1;

        public delegate void ErrorHandler(string message);
        private readonly ErrorHandler errorHandler;

        public WatermarkHelper(ErrorHandler errorHandler) => this.errorHandler = errorHandler;

        public static readonly Dictionary<string, ImageFormat> extensionToImageFormat = new() {
            { ".bmp", ImageFormat.Bmp },
            { ".png", ImageFormat.Png },
            { ".jpeg", ImageFormat.Jpeg },
            { ".jpg", ImageFormat.Jpeg },
            { ".gif", ImageFormat.Gif }
        };

        #region Watermark Methods
        public Bitmap? AddWatermark(Bitmap originalImage, string watermarkText)
        {
            if (originalImage == null || string.IsNullOrEmpty(watermarkText)) {
                errorHandler?.Invoke("Invalid image or watermark text.");
                return null;
            }

            Bitmap watermarkedImage = new(originalImage);
            ApplyWatermarkToImage(watermarkedImage, watermarkText);
            return watermarkedImage;
        }

        private unsafe static void ApplyWatermarkToImage(Bitmap image, string watermarkText)
        {
            Rectangle imageRect = new(0, 0, image.Width, image.Height);
            BitmapData imageData = image.LockBits(imageRect, ImageLockMode.ReadWrite, image.PixelFormat);

            try {
                int bytesPerPixel = Image.GetPixelFormatSize(image.PixelFormat) / 8;
                int imageHeight = imageData.Height;
                int imageWidth = imageData.Width;
                Span<byte> pixelDataSpan = new((byte*)imageData.Scan0, imageHeight * imageData.Stride);
                byte[] pixelData = pixelDataSpan.ToArray();

                EmbedWatermarkIntoPixels(pixelData, imageWidth, imageData.Stride, watermarkText, bytesPerPixel);

                pixelDataSpan = pixelData;
                Marshal.Copy(pixelData, 0, (IntPtr)imageData.Scan0, pixelData.Length);
            }
            finally {
                image.UnlockBits(imageData);
            }
        }

        private static void EmbedWatermarkIntoPixels(byte[] pixels, int width, int stride, string watermarkText, int bytesPerPixel)
        {
            Parallel.For(0, watermarkText.Length, i => {
                char character = watermarkText[i];
                byte characterValue = (byte)character;

                for (int bit = 0; bit < BitsPerByte; bit++) {
                    int x = (i * BitsPerByte + bit) % width;
                    int y = (i * BitsPerByte + bit) / width;

                    if (y >= stride / bytesPerPixel)
                        break;

                    int pixelIndex = y * stride + x * bytesPerPixel;
                    pixels[pixelIndex] = (byte)((pixels[pixelIndex] & ~LeastSignificantBitMask) | ((characterValue >> bit) & LeastSignificantBitMask));
                }
            });
        }
        #endregion

        #region Directory Processing Methods
        public async Task AddWatermarkToDirectoryAsync(string sourceDirectory, string targetDirectory, string watermarkText)
        {
            if (!Directory.Exists(sourceDirectory)) {
                errorHandler?.Invoke($"Source directory '{sourceDirectory}' not found.");
                return;
            }
            if (!Directory.Exists(targetDirectory))
                Directory.CreateDirectory(targetDirectory);

            if (string.IsNullOrWhiteSpace(watermarkText)) {
                errorHandler?.Invoke("Watermark text cannot be null or whitespace.");
                return;
            }

            var imageFiles = GetImageFilesFromDirectory(sourceDirectory);

            var tasks = imageFiles.Select(file => ProcessImageAsync(file, targetDirectory, watermarkText));
            await Task.WhenAll(tasks);
        }

        private static IEnumerable<string> GetImageFilesFromDirectory(string directoryPath)
        {
            return Directory.EnumerateFiles(directoryPath, "*.*", SearchOption.TopDirectoryOnly)
                .Where(file => file.EndsWith(".jpg") || file.EndsWith(".jpeg") || file.EndsWith(".png") || file.EndsWith(".bmp") || file.EndsWith(".gif"));
        }

        private async Task ProcessImageAsync(string filePath, string targetDirectory, string watermarkText)
        {
            try {
                using Bitmap originalImage = new(filePath);
                Bitmap? watermarkedImage = AddWatermark(originalImage, watermarkText);
                if (watermarkedImage != null) {
                    string fileName = Path.GetFileName(filePath);
                    string targetPath = Path.Combine(targetDirectory, fileName);
                    ImageFormat imageFormat = GetImageFormat(filePath);
                    await Task.Run(() => SaveImage(watermarkedImage, targetPath, imageFormat));
                }
                else
                    errorHandler?.Invoke($"Failed to add watermark to {filePath}.");
            }
            catch (Exception ex) {
                errorHandler?.Invoke($"Error processing {filePath}: {ex.Message}");
            }
        }
        #endregion

        #region Image Saving Methods
        private void SaveImage(Bitmap image, string filePath, ImageFormat imageFormat)
        {
            if (image == null || string.IsNullOrWhiteSpace(filePath)) {
                errorHandler?.Invoke("Invalid image or path for saving.");
                return;
            }

            try {
                image.Save(filePath, imageFormat);
            }
            catch (Exception ex) {
                errorHandler?.Invoke($"Failed to save watermarked image to {filePath}: {ex.Message}");
            }
        }

        private static ImageFormat GetImageFormat(string filePath)
        {
            string fileExtension = Path.GetExtension(filePath).ToLower();
            if (extensionToImageFormat.TryGetValue(fileExtension, out var imageFormat))
                return imageFormat;
            throw new ArgumentException("Unsupported image format.");
        }
        #endregion

        #region Watermark Extraction Methods
        public string ExtractWatermark(Bitmap image)
        {
            if (image == null) {
                errorHandler?.Invoke("Invalid image for watermark extraction.");
                return "";
            }

            return ExtractWatermarkText(image);
        }

        private static string ExtractWatermarkText(Bitmap image)
        {
            StringBuilder watermarkText = new();
            int imageWidth = image.Width;
            int imageHeight = image.Height;
            int bytesPerPixel = Image.GetPixelFormatSize(image.PixelFormat) / 8;
            int maxLength = (imageWidth * imageHeight * bytesPerPixel) / BitsPerByte;

            for (int i = 0; i < maxLength; i++) {
                byte characterValue = 0;
                for (int bit = 0; bit < BitsPerByte; bit++) {
                    int x = (i * BitsPerByte + bit) % imageWidth;
                    int y = (i * BitsPerByte + bit) / imageWidth;

                    if (y >= imageHeight)
                        break;

                    Color pixelColor = image.GetPixel(x, y);
                    byte bitValue = (byte)(pixelColor.B & LeastSignificantBitMask);
                    characterValue |= (byte)(bitValue << bit);
                }

                if (characterValue == 0)
                    break;
                if (characterValue >= MinPrintableAscii && characterValue <= MaxPrintableAscii)
                    watermarkText.Append((char)characterValue);
                else
                    break;
            }

            return watermarkText.ToString();
        }
        #endregion
    }
}