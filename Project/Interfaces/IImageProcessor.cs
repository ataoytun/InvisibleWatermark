using System;
using System.IO;
using System.Threading;
using System.Threading.Tasks;
using InvisibleWatermark.Enums;

namespace InvisibleWatermark.Interfaces
{
    public interface IImageProcessor : IDisposable
    {
        Task ApplyWatermarkAsync(Stream inputStream, Stream outputStream, string watermarkText, EColorChannel channel, CancellationToken cancellationToken, IProgress<double> progress);
        Task<string> ExtractWatermarkAsync(Stream inputStream, EColorChannel channel, CancellationToken cancellationToken, IProgress<double> progress);
    }
}