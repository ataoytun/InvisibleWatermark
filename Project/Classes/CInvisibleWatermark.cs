using InvisibleWatermark.Enums;
using InvisibleWatermark.Interfaces;
using InvisibleWatermark.Structs;
using System;
using System.Collections.Concurrent;
using System.IO;
using System.Threading;
using System.Threading.Tasks;

namespace InvisibleWatermark.Classes
{
    public class CInvisibleWatermark : IDisposable
    {
        private readonly IImageProcessor _imageProcessor;
        private readonly IFileSystem _fileSystem;
        private readonly BlockingCollection<SWatermarkJob> _jobQueue;
        private readonly CancellationTokenSource _cts;
        private readonly Task[] _processingTasks;
        private bool _disposed = false;

        public CInvisibleWatermark(IImageProcessor imageProcessor, IFileSystem fileSystem, int workerCount = 4)
        {
            _imageProcessor = imageProcessor ?? throw new ArgumentNullException(nameof(imageProcessor));
            _fileSystem = fileSystem ?? throw new ArgumentNullException(nameof(fileSystem));
            _jobQueue = new BlockingCollection<SWatermarkJob>();
            _cts = new CancellationTokenSource();
            _processingTasks = new Task[workerCount];

            for (int i = 0; i < workerCount; i++)
                _processingTasks[i] = Task.Run(() => ProcessJobsAsync(_cts.Token));
        }

        public Task AddWatermarkAsync(string inputPath, string outputPath, string watermarkText, EColorChannel channel = EColorChannel.Red, IProgress<double> progress = null)
        {
            ValidateInputs(inputPath, outputPath, watermarkText);
            var job = new SWatermarkJob(inputPath, outputPath, watermarkText, channel, progress);
            _jobQueue.Add(job);
            return job.CompletionTask;
        }

        private async Task ProcessJobsAsync(CancellationToken cancellationToken)
        {
            while (!cancellationToken.IsCancellationRequested) {
                SWatermarkJob job;
                try {
                    job = _jobQueue.Take(cancellationToken);
                }
                catch (OperationCanceledException)
                {
                    break;
                }

                try {
                    await ProcessJobAsync(job, cancellationToken);
                }
                catch (Exception ex) {
                    Console.WriteLine($"Error processing watermark job: {ex.Message}");
                    job.SetException(ex);
                }
            }
        }

        private async Task ProcessJobAsync(SWatermarkJob job, CancellationToken cancellationToken)
        {
            try {
                using (var inputStream = await _fileSystem.OpenReadAsync(job.InputPath, cancellationToken))
                    using (var outputStream = await _fileSystem.OpenWriteAsync(job.OutputPath, cancellationToken)) {
                        await _imageProcessor.ApplyWatermarkAsync(inputStream, outputStream, job.WatermarkText, job.Channel, cancellationToken, job.Progress);
                    }
                job.SetResult();
            }
            catch (IOException ex) {
                throw new CWatermarkException("Error accessing image file.", ex);
            }
            catch (CImageProcessingException ex) {
                throw new CWatermarkException("Error processing image.", ex);
            }
        }

        public async Task<string> RetrieveWatermarkAsync(string imagePath, EColorChannel channel = EColorChannel.Red, CancellationToken cancellationToken = default, IProgress<double> progress = null)
        {
            if (string.IsNullOrEmpty(imagePath))
                throw new ArgumentNullException(nameof(imagePath), "Image path cannot be null or empty.");

            if (!await _fileSystem.FileExistsAsync(imagePath, cancellationToken))
                throw new FileNotFoundException("Image file not found.", imagePath);

            try {
                using (var inputStream = await _fileSystem.OpenReadAsync(imagePath, cancellationToken)) {
                    return await _imageProcessor.ExtractWatermarkAsync(inputStream, channel, cancellationToken, progress);
                }
            }
            catch (IOException ex) {
                throw new CWatermarkException("Error accessing image file.", ex);
            }
            catch (CImageProcessingException ex) {
                throw new CWatermarkException("Error processing image.", ex);
            }
        }

        private void ValidateInputs(string inputPath, string outputPath, string watermarkText)
        {
            if (string.IsNullOrEmpty(inputPath))
                throw new ArgumentNullException(nameof(inputPath), "Input path cannot be null or empty.");
            if (string.IsNullOrEmpty(outputPath))
                throw new ArgumentNullException(nameof(outputPath), "Output path cannot be null or empty.");
            if (string.IsNullOrEmpty(watermarkText))
                throw new ArgumentNullException(nameof(watermarkText), "Watermark text cannot be null or empty.");

            if (!_fileSystem.FileExists(inputPath))
                throw new FileNotFoundException("Input image file not found.", inputPath);
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
                    _cts.Cancel();
                    Task.WhenAll(_processingTasks).Wait();
                    _jobQueue.Dispose();
                    _cts.Dispose();

                    (_imageProcessor as IDisposable)?.Dispose();
                    (_fileSystem as IDisposable)?.Dispose();
                }
                _disposed = true;
            }
        }
    }
}