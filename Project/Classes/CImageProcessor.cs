using InvisibleWatermark.Enums;
using InvisibleWatermark.Interfaces;
using System;
using System.Drawing;
using System.Drawing.Imaging;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading;
using System.Threading.Tasks;

namespace InvisibleWatermark.Classes
{
    public class CImageProcessor : IImageProcessor
    {
        private const int BitsPerChar = 8;
        private bool _disposed = false;

        public async Task ApplyWatermarkAsync(Stream inputStream, Stream outputStream, string watermarkText, EColorChannel channel, CancellationToken cancellationToken, IProgress<double> progress)
        {
            using (var image = Image.FromStream(inputStream)) {
                var bitmap = new Bitmap(image);

                int watermarkLength = watermarkText.Length * BitsPerChar;
                int totalPixels = bitmap.Width * bitmap.Height;
                int processedPixels = 0;

                BitmapData bmpData = bitmap.LockBits(new Rectangle(0, 0, bitmap.Width, bitmap.Height), ImageLockMode.ReadWrite, bitmap.PixelFormat);

                try {
                    int bytesPerPixel = Image.GetPixelFormatSize(bitmap.PixelFormat) / 8;
                    int stride = bmpData.Stride;
                    IntPtr ptr = bmpData.Scan0;

                    byte[] rgbValues = new byte[stride * bitmap.Height];
                    Marshal.Copy(ptr, rgbValues, 0, rgbValues.Length);

                    for (int y = 0; y < bitmap.Height; y++)
                        for (int x = 0; x < bitmap.Width; x++) {
                            cancellationToken.ThrowIfCancellationRequested();

                            int index = y * stride + x * bytesPerPixel;
                            int watermarkIndex = processedPixels % watermarkLength;
                            int watermarkBit = GetWatermarkBit(watermarkText, watermarkIndex);

                            ApplyWatermarkToPixel(rgbValues, index, channel, watermarkBit);

                            processedPixels++;
                            if (processedPixels % 1000 == 0 || processedPixels == totalPixels)
                                progress?.Report((double)processedPixels / totalPixels);
                        }

                    Marshal.Copy(rgbValues, 0, ptr, rgbValues.Length);
                }
                finally {
                    bitmap.UnlockBits(bmpData);
                }

                bitmap.Save(outputStream, image.RawFormat);
            }

            await Task.CompletedTask;
        }

        public async Task<string> ExtractWatermarkAsync(Stream inputStream, EColorChannel channel, CancellationToken cancellationToken, IProgress<double> progress)
        {
            using (var image = Image.FromStream(inputStream))
            {
                var bitmap = new Bitmap(image);

                StringBuilder watermarkBuilder = new StringBuilder();
                int bitCount = 0;
                byte currentByte = 0;
                int totalPixels = bitmap.Width * bitmap.Height;
                int processedPixels = 0;

                BitmapData bmpData = bitmap.LockBits(new Rectangle(0, 0, bitmap.Width, bitmap.Height), ImageLockMode.ReadOnly, bitmap.PixelFormat);

                try {
                    int bytesPerPixel = Image.GetPixelFormatSize(bitmap.PixelFormat) / 8;
                    int stride = bmpData.Stride;
                    IntPtr ptr = bmpData.Scan0;

                    byte[] rgbValues = new byte[stride * bitmap.Height];
                    Marshal.Copy(ptr, rgbValues, 0, rgbValues.Length);

                    for (int y = 0; y < bitmap.Height; y++)
                        for (int x = 0; x < bitmap.Width; x++) {
                            cancellationToken.ThrowIfCancellationRequested();

                            int index = y * stride + x * bytesPerPixel;
                            int bit = GetChannelBit(rgbValues, index, channel);

                            currentByte = (byte)((currentByte << 1) | bit);
                            bitCount++;

                            if (bitCount == BitsPerChar) {
                                if (currentByte == 0) {
                                    progress?.Report(1.0);
                                    return watermarkBuilder.ToString();
                                }

                                watermarkBuilder.Append((char)currentByte);
                                bitCount = 0;
                                currentByte = 0;
                            }

                            processedPixels++;
                            if (processedPixels % 1000 == 0 || processedPixels == totalPixels)
                                progress?.Report((double)processedPixels / totalPixels);
                        }
                }
                finally {
                    bitmap.UnlockBits(bmpData);
                }

                progress?.Report(1.0);
                return watermarkBuilder.ToString();
            }
        }

        private static void ApplyWatermarkToPixel(byte[] pixel, int offset, EColorChannel channel, int watermarkBit)
        {
            int channelIndex = GetChannelIndex(channel);
            pixel[offset + channelIndex] = (byte)((pixel[offset + channelIndex] & 0xFE) | watermarkBit);
        }

        private static int GetChannelBit(byte[] pixel, int offset, EColorChannel channel)
        {
            int channelIndex = GetChannelIndex(channel);
            return pixel[offset + channelIndex] & 1;
        }

        private static int GetChannelIndex(EColorChannel channel)
        {
            switch (channel) {
                case EColorChannel.Blue:
                    return 0;
                case EColorChannel.Green:
                    return 1;
                case EColorChannel.Red:
                    return 2;
                default:
                    throw new ArgumentException("Invalid color channel", nameof(channel));
            }
        }

        private static int GetWatermarkBit(string watermarkText, int index)
        {
            int charIndex = index / BitsPerChar;
            int bitIndex = index % BitsPerChar;

            if (charIndex >= watermarkText.Length)
                return 0;

            char c = watermarkText[charIndex];
            return (c >> (7 - bitIndex)) & 1;
        }

        public void Dispose()
        {
            Dispose(true);
            GC.SuppressFinalize(this);
        }

        protected virtual void Dispose(bool disposing)
        {
            if (!_disposed) {
                if (disposing) {
                
                }

                _disposed = true;
            }
        }
    }
}