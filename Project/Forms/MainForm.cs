using InvisibleWatermark.Classes;
using InvisibleWatermark.Enums;
using System;
using System.Data;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Media;
using System.Threading.Tasks;
using System.Windows.Forms;

namespace InvisibleWatermark
{
    public partial class MainForm : Form
    {
        private readonly CInvisibleWatermark _invisibleWatermark;

        private const string OUTPUT_FOLDER = "Output";
        private static readonly string[] SUPPORTED_EXTENSIONS = { ".jpg", ".jpeg", ".png", ".bmp" };
        private const string GITHUB_URL = "https://github.com/ataoytun/InvisibleWatermark";
        private const int MAXIUM_WATERMARK_LENGTH = 50;

        public MainForm()
        {
            InitializeComponent();
            _invisibleWatermark = new CInvisibleWatermark(new CImageProcessor(), new CFileSystem());
        }

        private async void btnAddWatermark_Click(object sender, EventArgs e)
        {
            using (OpenFileDialog openFileDialog = new OpenFileDialog()) {
                if (openFileDialog.ShowDialog() == DialogResult.OK)
                    await ProcessSingleImageAsync(openFileDialog.FileName);
            }
        }

        private async void btnAddWatermarkToDirectory_Click(object sender, EventArgs e)
        {
            using (FolderBrowserDialog folderBrowserDialog = new FolderBrowserDialog()) {
                if (folderBrowserDialog.ShowDialog() == DialogResult.OK)
                    await ProcessDirectoryAsync(folderBrowserDialog.SelectedPath);
            }
        }

        private async Task ProcessSingleImageAsync(string inputPath)
        {
            string outputPath = GetOutputPath(inputPath);
            await ProcessImageAsync(inputPath, outputPath);
        }

        private async Task ProcessDirectoryAsync(string directoryPath)
        {
            string[] imageFiles = Directory.GetFiles(directoryPath, "*.*", SearchOption.AllDirectories)
                .Where(file => SUPPORTED_EXTENSIONS.Contains(Path.GetExtension(file).ToLower()))
                .ToArray();

            foreach (string inputPath in imageFiles) {
                string outputPath = GetOutputPath(inputPath);
                await ProcessImageAsync(inputPath, outputPath);
            }

            ShowSuccessMessage("Watermarks added to all images in the directory!");
        }

        private async Task ProcessImageAsync(string inputPath, string outputPath)
        {
            try {
                EnsureOutputDirectoryExists(outputPath);
                var progress = new Progress<double>(UpdateProgressBar);
                await _invisibleWatermark.AddWatermarkAsync(inputPath, outputPath, txtWatermark.Text, EColorChannel.Red, progress);
            }
            catch (Exception ex) {
                HandleException(ex);
            }
        }

        private void EnsureOutputDirectoryExists(string outputPath)
        {
            Directory.CreateDirectory(Path.GetDirectoryName(outputPath));
        }

        private string GetOutputPath(string inputPath)
        {
            return Path.Combine(Application.StartupPath, OUTPUT_FOLDER, $"{Path.GetFileNameWithoutExtension(inputPath)}_watermarked.png");
        }

        private void UpdateProgressBar(double value)
        {
            if (InvokeRequired) {
                BeginInvoke(new Action<double>(UpdateProgressBar), value);
                return;
            }
            pbarProgress.Value = (int)(value * 100);
        }

        private void HandleException(Exception ex)
        {
            string message;
            if (ex is ArgumentNullException)
                message = ex.Message;
            else if (ex is CWatermarkException)
                message = $"Error adding watermark: {ex.Message}";
            else if (ex is CImageProcessingException)
                message = $"Error processing image: {ex.Message}";
            else
                message = $"An unexpected error occurred: {ex.Message}";

            SystemSounds.Hand.Play();
            MessageBox.Show(message, "Error", MessageBoxButtons.OK, MessageBoxIcon.Error);
        }

        private void ShowSuccessMessage(string message)
        {
            SystemSounds.Asterisk.Play();
            MessageBox.Show(message, "Success", MessageBoxButtons.OK, MessageBoxIcon.Information);
        }

        private async void btnRetrieveWatermark_Click(object sender, EventArgs e)
        {
            using (OpenFileDialog openFileDialog = new OpenFileDialog()) {
                if (openFileDialog.ShowDialog() == DialogResult.OK)
                    await RetrieveWatermarkAsync(openFileDialog.FileName);
            }
        }

        private async Task RetrieveWatermarkAsync(string filePath)
        {
            var progress = new Progress<double>(UpdateProgressBar);
            try {
                var retrievedWatermark = await _invisibleWatermark.RetrieveWatermarkAsync(filePath, EColorChannel.Red, default, progress);
                retrievedWatermark = retrievedWatermark.Substring(0, MAXIUM_WATERMARK_LENGTH);
                ShowSuccessMessage($"Watermark retrieved successfully!\n{retrievedWatermark}");
            }
            catch (Exception ex) {
                HandleException(ex);
            }
        }

        private void picGithub_Click(object sender, EventArgs e)
        {
            if (ConfirmNavigation())
                Process.Start(new ProcessStartInfo { FileName = GITHUB_URL, UseShellExecute = true });
        }

        private bool ConfirmNavigation()
        {
            return MessageBox.Show("Do you want to go to the project's GitHub page?", "Go to GitHub",
                MessageBoxButtons.YesNo, MessageBoxIcon.Question) == DialogResult.Yes;
        }
    }
}