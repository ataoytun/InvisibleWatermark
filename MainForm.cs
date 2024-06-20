using System.ComponentModel;
using System.Drawing.Imaging;
using Microsoft.VisualBasic;
#pragma warning disable CA1416 // Ignores the warning for '....' which is only supported on Windows 6.1 and later.

namespace InvisibleWatermark
{
    public partial class MainForm : Form
    {
        #region Fields
        private Bitmap? loadedImage;
        private readonly WatermarkHelper watermarkHelper;
        private string currentWatermark = "Sample Watermark";
        private const string ImageFilter = "Image Files|*.jpg;*.jpeg;*.png;";
        private const string WatermarkPromptTitle = "Change Watermark";
        private const string WatermarkPromptMessage = "Enter the watermark text: \nCurrent Watermark: ";
        #endregion

        #region Constructor
        public MainForm()
        {
            InitializeComponent();
            watermarkHelper = new WatermarkHelper(HandleError);
        }
        #endregion

        #region Error Handling
        private void HandleError(string message)
        {
            if (InvokeRequired)
                BeginInvoke(new WatermarkHelper.ErrorHandler(HandleError), message);
            else
                richTextBoxLogs.AppendText(message + Environment.NewLine);
        }
        #endregion

        #region Menu Item Handlers
        private async void menuItemSingleImage_Click(object sender, EventArgs e)
        {
            await LoadSingleImageAsync();
        }

        private void menuItemSelectFolder_Click(object sender, EventArgs e)
        {
            SelectFoldersAndStartProcessing();
        }

        private void menuItemChangeWatermark_Click(object sender, EventArgs e)
        {
            ChangeWatermark();
        }

        private void menuItemExtractWatermark_Click(object sender, EventArgs e)
        {
            ExtractWatermark();
        }
        #endregion

        #region Image Loading
        private async Task LoadSingleImageAsync()
        {
            try {
                using OpenFileDialog openFileDialog = new() { Filter = ImageFilter };

                if (openFileDialog.ShowDialog() == DialogResult.OK) {
                    await using var stream = new FileStream(openFileDialog.FileName, FileMode.Open, FileAccess.Read, FileShare.Read, 4096, true);
                    loadedImage?.Dispose();
                    loadedImage = new Bitmap(stream);
                    pictureBoxPreview.Image = loadedImage;
                }
            }
            catch (Exception ex) {
                HandleError($"Error loading image: {ex.Message}");
            }
        }
        #endregion

        #region Folder Processing
        private void SelectFoldersAndStartProcessing()
        {
            try {
                using FolderBrowserDialog sourceFolderDialog = new() { Description = "Select Input Folder" };

                if (sourceFolderDialog.ShowDialog() == DialogResult.OK) {
                    string sourceDirectory = sourceFolderDialog.SelectedPath;
                    using FolderBrowserDialog targetFolderDialog = new() { Description = "Select Output Folder" };

                    if (targetFolderDialog.ShowDialog() == DialogResult.OK) {
                        string targetDirectory = targetFolderDialog.SelectedPath;

                        if (!backgroundWorkerBulkOperation.IsBusy) {
                            progressBarStatus.Value = 0;
                            progressBarStatus.Style = ProgressBarStyle.Marquee;
                            backgroundWorkerBulkOperation.RunWorkerAsync(new Tuple<string, string, string>(sourceDirectory, targetDirectory, currentWatermark));
                        }
                    }
                }
            }
            catch (Exception ex) {
                HandleError($"Error selecting folders: {ex.Message}");
            }
        }
        #endregion

        #region Watermark Handling
        private void ChangeWatermark()
        {
            try {
                string watermarkInput = Interaction.InputBox(WatermarkPromptMessage + currentWatermark, WatermarkPromptTitle, currentWatermark, 0, 0);
                if (string.IsNullOrWhiteSpace(watermarkInput)) {
                    MessageBox.Show("Watermark text cannot be empty.", "Validation Error", MessageBoxButtons.OK, MessageBoxIcon.Warning);
                    return;
                }
                currentWatermark = watermarkInput;
            }
            catch (Exception ex) {
                HandleError($"Error changing watermark text: {ex.Message}");
            }
        }

        private void ExtractWatermark()
        {
            try {
                if (loadedImage == null) {
                    MessageBox.Show("Please load an image first.", "Warning", MessageBoxButtons.OK, MessageBoxIcon.Warning);
                    return;
                }

                string watermarkText = watermarkHelper.ExtractWatermark(loadedImage);
                MessageBox.Show($"Extracted Watermark: {watermarkText}", "Watermark Extracted", MessageBoxButtons.OK, MessageBoxIcon.Information);
            }
            catch (Exception ex) {
                HandleError($"Error extracting watermark: {ex.Message}");
            }
        }
        #endregion

        #region Background Worker Handlers
        private void backgroundWorkerBulkOperation_DoWork(object sender, DoWorkEventArgs e)
        {
            var args = (Tuple<string, string, string>?)e.Argument;
            if (args == null) {
                HandleError("Invalid arguments provided to background worker.");
                return;
            }

            string sourceDirectory = args.Item1;
            string targetDirectory = args.Item2;
            string watermarkText = args.Item3;

            var imageFiles = GetImageFiles(sourceDirectory).ToList();
            int totalFiles = imageFiles.Count;
            int processedFiles = 0;

            var progress = new Progress<int>(value => backgroundWorkerBulkOperation.ReportProgress(value));

            var task = Task.Run(() => {
                Parallel.ForEach(imageFiles, new ParallelOptions { MaxDegreeOfParallelism = Environment.ProcessorCount }, (file, state) => {
                    if (backgroundWorkerBulkOperation.CancellationPending)
                        state.Break();

                    try {
                        ProcessSingleImage(file, targetDirectory, watermarkText);
                        processedFiles++;
                        int progressPercentage = (int)((double)processedFiles / totalFiles * 100);
                        ((IProgress<int>)progress).Report(progressPercentage);
                    }
                    catch (Exception ex) {
                        HandleError($"Error processing file {file}: {ex.Message}");
                    }
                });
            });

            task.Wait();
            e.Result = processedFiles;
        }

        private void backgroundWorkerBulkOperation_ProgressChanged(object sender, ProgressChangedEventArgs e)
        {
            UpdateProgressBar(e.ProgressPercentage);
        }

        private void backgroundWorkerBulkOperation_RunWorkerCompleted(object sender, RunWorkerCompletedEventArgs e)
        {
            if (e.Error != null)
                HandleError(e.Error.Message);
            else if (e.Result != null) {
                int processedFiles = (int)e.Result;
                MessageBox.Show($"Processing completed. {processedFiles} files processed.", "Completed", MessageBoxButtons.OK, MessageBoxIcon.Information);
            }

            progressBarStatus.Value = 100;
            progressBarStatus.Style = ProgressBarStyle.Blocks;
        }
        #endregion

        #region Helper Methods
        private IEnumerable<string> GetImageFiles(string directory)
        {
            return Directory.EnumerateFiles(directory, "*.*", SearchOption.TopDirectoryOnly)
                .Where(file => WatermarkHelper.extensionToImageFormat.ContainsKey(Path.GetExtension(file).ToLower()));
        }

        private void ProcessSingleImage(string filePath, string targetDirectory, string watermarkText)
        {
            using var stream = new FileStream(filePath, FileMode.Open, FileAccess.Read, FileShare.Read, 4096, true);
            using Bitmap originalImage = new(stream);
            Bitmap? watermarkedImage = watermarkHelper.AddWatermark(originalImage, watermarkText);
            if (watermarkedImage != null) {
                string fileName = Path.GetFileName(filePath);
                string targetPath = Path.Combine(targetDirectory, fileName);
                ImageFormat imageFormat = GetImageFormat(filePath);
                watermarkedImage.Save(targetPath, imageFormat);
                watermarkedImage.Dispose();
            }
        }

        private ImageFormat GetImageFormat(string filePath)
        {
            string fileExtension = Path.GetExtension(filePath).ToLower();
            if (WatermarkHelper.extensionToImageFormat.TryGetValue(fileExtension, out var imageFormat))
                return imageFormat;

            throw new ArgumentException("Unsupported image format.");
        }

        private void UpdateProgressBar(int progressPercentage)
        {
            if (InvokeRequired)
                Invoke(new Action<int>(UpdateProgressBar), progressPercentage);
            else {
                progressBarStatus.Style = ProgressBarStyle.Blocks;
                progressBarStatus.Value = progressPercentage;
            }
        }
        #endregion
    }
}