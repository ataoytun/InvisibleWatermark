using System;
using System.IO;
using System.Threading;
using System.Threading.Tasks;

namespace InvisibleWatermark.Interfaces
{
    public interface IFileSystem : IDisposable
    {
        bool FileExists(string path);
        Task<bool> FileExistsAsync(string path, CancellationToken cancellationToken);
        Task<Stream> OpenReadAsync(string path, CancellationToken cancellationToken);
        Task<Stream> OpenWriteAsync(string path, CancellationToken cancellationToken);
    }
}