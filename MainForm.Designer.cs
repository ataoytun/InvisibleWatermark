namespace InvisibleWatermark
{
    partial class MainForm
    {
        /// <summary>
        ///  Required designer variable.
        /// </summary>
        private System.ComponentModel.IContainer components = null;

        /// <summary>
        ///  Clean up any resources being used.
        /// </summary>
        /// <param name="disposing">true if managed resources should be disposed; otherwise, false.</param>
        protected override void Dispose(bool disposing)
        {
            if (disposing && (components != null))
            {
                components.Dispose();
            }
            base.Dispose(disposing);
        }

        #region Windows Form Designer generated code

        /// <summary>
        ///  Required method for Designer support - do not modify
        ///  the contents of this method with the code editor.
        /// </summary>
        private void InitializeComponent()
        {
            System.ComponentModel.ComponentResourceManager resources = new System.ComponentModel.ComponentResourceManager(typeof(MainForm));
            menuStrip = new MenuStrip();
            menuItemFile = new ToolStripMenuItem();
            menuItemSingleImage = new ToolStripMenuItem();
            menuItemSelectFolder = new ToolStripMenuItem();
            menuItemWatermark = new ToolStripMenuItem();
            menuItemChangeWatermark = new ToolStripMenuItem();
            menuItemExtractWatermark = new ToolStripMenuItem();
            pictureBoxPreview = new PictureBox();
            progressBarStatus = new ProgressBar();
            richTextBoxLogs = new RichTextBox();
            backgroundWorkerBulkOperation = new System.ComponentModel.BackgroundWorker();
            menuStrip.SuspendLayout();
            ((System.ComponentModel.ISupportInitialize)pictureBoxPreview).BeginInit();
            SuspendLayout();
            // 
            // menuStrip
            // 
            menuStrip.Items.AddRange(new ToolStripItem[] { menuItemFile, menuItemWatermark });
            menuStrip.Location = new Point(0, 0);
            menuStrip.Name = "menuStrip";
            menuStrip.Size = new Size(466, 24);
            menuStrip.TabIndex = 0;
            menuStrip.Text = "menuStrip";
            // 
            // menuItemFile
            // 
            menuItemFile.DropDownItems.AddRange(new ToolStripItem[] { menuItemSingleImage, menuItemSelectFolder });
            menuItemFile.Name = "menuItemFile";
            menuItemFile.Size = new Size(37, 20);
            menuItemFile.Text = "File";
            // 
            // menuItemSingleImage
            // 
            menuItemSingleImage.Name = "menuItemSingleImage";
            menuItemSingleImage.Size = new Size(141, 22);
            menuItemSingleImage.Text = "Load Image";
            menuItemSingleImage.Click += menuItemSingleImage_Click;
            // 
            // menuItemSelectFolder
            // 
            menuItemSelectFolder.Name = "menuItemSelectFolder";
            menuItemSelectFolder.Size = new Size(141, 22);
            menuItemSelectFolder.Text = "Select Folder";
            menuItemSelectFolder.Click += menuItemSelectFolder_Click;
            // 
            // menuItemWatermark
            // 
            menuItemWatermark.DropDownItems.AddRange(new ToolStripItem[] { menuItemChangeWatermark, menuItemExtractWatermark });
            menuItemWatermark.Name = "menuItemWatermark";
            menuItemWatermark.Size = new Size(77, 20);
            menuItemWatermark.Text = "Watermark";
            // 
            // menuItemChangeWatermark
            // 
            menuItemChangeWatermark.Name = "menuItemChangeWatermark";
            menuItemChangeWatermark.Size = new Size(176, 22);
            menuItemChangeWatermark.Text = "Change Watermark";
            menuItemChangeWatermark.Click += menuItemChangeWatermark_Click;
            // 
            // menuItemExtractWatermark
            // 
            menuItemExtractWatermark.Name = "menuItemExtractWatermark";
            menuItemExtractWatermark.Size = new Size(176, 22);
            menuItemExtractWatermark.Text = "Extract Watermark";
            menuItemExtractWatermark.Click += menuItemExtractWatermark_Click;
            // 
            // pictureBoxPreview
            // 
            pictureBoxPreview.Location = new Point(0, 27);
            pictureBoxPreview.Name = "pictureBoxPreview";
            pictureBoxPreview.Size = new Size(226, 261);
            pictureBoxPreview.SizeMode = PictureBoxSizeMode.Zoom;
            pictureBoxPreview.TabIndex = 1;
            pictureBoxPreview.TabStop = false;
            // 
            // progressBarStatus
            // 
            progressBarStatus.Location = new Point(232, 265);
            progressBarStatus.Name = "progressBarStatus";
            progressBarStatus.Size = new Size(226, 23);
            progressBarStatus.TabIndex = 2;
            // 
            // richTextBoxLogs
            // 
            richTextBoxLogs.Font = new Font("Segoe UI Light", 9F, FontStyle.Regular, GraphicsUnit.Point, 0);
            richTextBoxLogs.Location = new Point(232, 27);
            richTextBoxLogs.Name = "richTextBoxLogs";
            richTextBoxLogs.ReadOnly = true;
            richTextBoxLogs.Size = new Size(226, 232);
            richTextBoxLogs.TabIndex = 3;
            richTextBoxLogs.Text = "";
            // 
            // backgroundWorkerBulkOperation
            // 
            backgroundWorkerBulkOperation.WorkerReportsProgress = true;
            backgroundWorkerBulkOperation.WorkerSupportsCancellation = true;
            backgroundWorkerBulkOperation.DoWork += backgroundWorkerBulkOperation_DoWork;
            backgroundWorkerBulkOperation.ProgressChanged += backgroundWorkerBulkOperation_ProgressChanged;
            backgroundWorkerBulkOperation.RunWorkerCompleted += backgroundWorkerBulkOperation_RunWorkerCompleted;
            // 
            // MainForm
            // 
            AutoScaleDimensions = new SizeF(7F, 15F);
            AutoScaleMode = AutoScaleMode.Font;
            BackColor = Color.FromArgb(192, 197, 206);
            ClientSize = new Size(466, 292);
            Controls.Add(richTextBoxLogs);
            Controls.Add(progressBarStatus);
            Controls.Add(pictureBoxPreview);
            Controls.Add(menuStrip);
            Icon = (Icon)resources.GetObject("$this.Icon");
            MainMenuStrip = menuStrip;
            MaximumSize = new Size(482, 331);
            MinimumSize = new Size(482, 331);
            Name = "MainForm";
            Text = "Invisible Watermark";
            menuStrip.ResumeLayout(false);
            menuStrip.PerformLayout();
            ((System.ComponentModel.ISupportInitialize)pictureBoxPreview).EndInit();
            ResumeLayout(false);
            PerformLayout();
        }

        #endregion

        private MenuStrip menuStrip;
        private ToolStripMenuItem menuItemFile;
        private ToolStripMenuItem menuItemSingleImage;
        private ToolStripMenuItem menuItemSelectFolder;
        private PictureBox pictureBoxPreview;
        private ProgressBar progressBarStatus;
        private RichTextBox richTextBoxLogs;
        private ToolStripMenuItem menuItemWatermark;
        private ToolStripMenuItem menuItemChangeWatermark;
        private ToolStripMenuItem menuItemExtractWatermark;
        private System.ComponentModel.BackgroundWorker backgroundWorkerBulkOperation;
    }
}
