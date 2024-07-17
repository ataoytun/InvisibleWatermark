using InvisibleWatermark.Interfaces;
using System;
using System.IO;
using System.Threading;
using System.Threading.Tasks;

namespace InvisibleWatermark.Classes
{
    public class CFileSystem : IFileSystem
    {
        private bool _disposed = false;

        public bool FileExists(string path) => File.Exists(path);

        public Task<bool> FileExistsAsync(string path, CancellationToken cancellationToken)
        {
            return Task.Run(() => File.Exists(path), cancellationToken);
        }

        public async Task<Stream> OpenReadAsync(string path, CancellationToken cancellationToken)
        {
            return await Task.Run(() => (Stream)new FileStream(path, FileMode.Open, FileAccess.Read, FileShare.Read, 4096, true), cancellationToken);
        }

        public async Task<Stream> OpenWriteAsync(string path, CancellationToken cancellationToken)
        {
            return await Task.Run(() => (Stream)new FileStream(path, FileMode.Create, FileAccess.Write, FileShare.None, 4096, true), cancellationToken);
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