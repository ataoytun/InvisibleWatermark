using System;
using System.Threading.Tasks;
using InvisibleWatermark.Enums;

namespace InvisibleWatermark.Structs
{
    public struct SWatermarkJob
    {
        public string InputPath { get; }
        public string OutputPath { get; }
        public string WatermarkText { get; }
        public EColorChannel Channel { get; }
        public IProgress<double> Progress { get; }
        private readonly TaskCompletionSource<bool> _completionSource;

        public Task CompletionTask => _completionSource.Task;

        public SWatermarkJob(string inputPath, string outputPath, string watermarkText, EColorChannel channel, IProgress<double> progress)
        {
            InputPath = inputPath;
            OutputPath = outputPath;
            WatermarkText = watermarkText;
            Channel = channel;
            Progress = progress;
            _completionSource = new TaskCompletionSource<bool>();
        }

        public void SetResult() => _completionSource.SetResult(true);
        public void SetException(Exception ex) => _completionSource.SetException(ex);
    }
}
