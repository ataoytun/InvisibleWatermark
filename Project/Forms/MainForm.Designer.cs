namespace InvisibleWatermark
{
    partial class MainForm
    {
        /// <summary>
        /// Required designer variable.
        /// </summary>
        private System.ComponentModel.IContainer components = null;

        /// <summary>
        /// Clean up any resources being used.
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
        /// Required method for Designer support - do not modify
        /// the contents of this method with the code editor.
        /// </summary>
        private void InitializeComponent()
        {
            this.components = new System.ComponentModel.Container();
            System.ComponentModel.ComponentResourceManager resources = new System.ComponentModel.ComponentResourceManager(typeof(MainForm));
            this.borderlessForm = new Guna.UI2.WinForms.Guna2BorderlessForm(this.components);
            this.pnlHeader = new Guna.UI2.WinForms.Guna2Panel();
            this.picLogo = new Guna.UI2.WinForms.Guna2PictureBox();
            this.label1 = new System.Windows.Forms.Label();
            this.cboxMinimize = new Guna.UI2.WinForms.Guna2ControlBox();
            this.cboxExit = new Guna.UI2.WinForms.Guna2ControlBox();
            this.dControlPanel = new Guna.UI2.WinForms.Guna2DragControl(this.components);
            this.gboxMain = new Guna.UI2.WinForms.Guna2GroupBox();
            this.label3 = new System.Windows.Forms.Label();
            this.pbarProgress = new Guna.UI2.WinForms.Guna2ProgressBar();
            this.label4 = new System.Windows.Forms.Label();
            this.btnRetrieveWatermark = new Guna.UI2.WinForms.Guna2Button();
            this.txtWatermark = new Guna.UI2.WinForms.Guna2TextBox();
            this.label2 = new System.Windows.Forms.Label();
            this.btnAddWatermarkToDirectory = new Guna.UI2.WinForms.Guna2Button();
            this.btnAddWatermark = new Guna.UI2.WinForms.Guna2Button();
            this.pnlBottom = new Guna.UI2.WinForms.Guna2Panel();
            this.picGithub = new Guna.UI2.WinForms.Guna2PictureBox();
            this.pnlHeader.SuspendLayout();
            ((System.ComponentModel.ISupportInitialize)(this.picLogo)).BeginInit();
            this.gboxMain.SuspendLayout();
            this.pnlBottom.SuspendLayout();
            ((System.ComponentModel.ISupportInitialize)(this.picGithub)).BeginInit();
            this.SuspendLayout();
            // 
            // borderlessForm
            // 
            this.borderlessForm.AnimationType = Guna.UI2.WinForms.Guna2BorderlessForm.AnimateWindowType.AW_CENTER;
            this.borderlessForm.ContainerControl = this;
            this.borderlessForm.DockIndicatorTransparencyValue = 0.6D;
            this.borderlessForm.DragForm = false;
            this.borderlessForm.ResizeForm = false;
            this.borderlessForm.TransparentWhileDrag = true;
            // 
            // pnlHeader
            // 
            this.pnlHeader.Controls.Add(this.picLogo);
            this.pnlHeader.Controls.Add(this.label1);
            this.pnlHeader.Controls.Add(this.cboxMinimize);
            this.pnlHeader.Controls.Add(this.cboxExit);
            this.pnlHeader.Dock = System.Windows.Forms.DockStyle.Top;
            this.pnlHeader.FillColor = System.Drawing.Color.FromArgb(((int)(((byte)(59)))), ((int)(((byte)(89)))), ((int)(((byte)(152)))));
            this.pnlHeader.Location = new System.Drawing.Point(0, 0);
            this.pnlHeader.Name = "pnlHeader";
            this.pnlHeader.Size = new System.Drawing.Size(258, 28);
            this.pnlHeader.TabIndex = 5;
            // 
            // picLogo
            // 
            this.picLogo.BackColor = System.Drawing.Color.Transparent;
            this.picLogo.Dock = System.Windows.Forms.DockStyle.Left;
            this.picLogo.Image = global::InvisibleWatermark.Properties.Resources.logo;
            this.picLogo.ImageRotate = 0F;
            this.picLogo.Location = new System.Drawing.Point(0, 0);
            this.picLogo.Name = "picLogo";
            this.picLogo.Size = new System.Drawing.Size(27, 28);
            this.picLogo.SizeMode = System.Windows.Forms.PictureBoxSizeMode.Zoom;
            this.picLogo.TabIndex = 6;
            this.picLogo.TabStop = false;
            this.picLogo.UseTransparentBackground = true;
            // 
            // label1
            // 
            this.label1.AutoSize = true;
            this.label1.BackColor = System.Drawing.Color.FromArgb(((int)(((byte)(59)))), ((int)(((byte)(89)))), ((int)(((byte)(152)))));
            this.label1.Font = new System.Drawing.Font("Segoe UI Light", 12F, System.Drawing.FontStyle.Regular, System.Drawing.GraphicsUnit.Point, ((byte)(0)));
            this.label1.ForeColor = System.Drawing.Color.FromArgb(((int)(((byte)(223)))), ((int)(((byte)(227)))), ((int)(((byte)(238)))));
            this.label1.Location = new System.Drawing.Point(28, 4);
            this.label1.Name = "label1";
            this.label1.Size = new System.Drawing.Size(138, 21);
            this.label1.TabIndex = 8;
            this.label1.Text = "Invisible Watermark";
            // 
            // cboxMinimize
            // 
            this.cboxMinimize.Animated = true;
            this.cboxMinimize.BackColor = System.Drawing.Color.Transparent;
            this.cboxMinimize.ControlBoxType = Guna.UI2.WinForms.Enums.ControlBoxType.MinimizeBox;
            this.cboxMinimize.Dock = System.Windows.Forms.DockStyle.Right;
            this.cboxMinimize.FillColor = System.Drawing.Color.FromArgb(((int)(((byte)(59)))), ((int)(((byte)(89)))), ((int)(((byte)(152)))));
            this.cboxMinimize.HoverState.FillColor = System.Drawing.Color.FromArgb(((int)(((byte)(188)))), ((int)(((byte)(159)))), ((int)(((byte)(130)))));
            this.cboxMinimize.IconColor = System.Drawing.Color.White;
            this.cboxMinimize.Location = new System.Drawing.Point(168, 0);
            this.cboxMinimize.Name = "cboxMinimize";
            this.cboxMinimize.Size = new System.Drawing.Size(45, 28);
            this.cboxMinimize.TabIndex = 7;
            // 
            // cboxExit
            // 
            this.cboxExit.Animated = true;
            this.cboxExit.BackColor = System.Drawing.Color.Transparent;
            this.cboxExit.Dock = System.Windows.Forms.DockStyle.Right;
            this.cboxExit.FillColor = System.Drawing.Color.FromArgb(((int)(((byte)(59)))), ((int)(((byte)(89)))), ((int)(((byte)(152)))));
            this.cboxExit.HoverState.FillColor = System.Drawing.Color.FromArgb(((int)(((byte)(159)))), ((int)(((byte)(22)))), ((int)(((byte)(22)))));
            this.cboxExit.IconColor = System.Drawing.Color.White;
            this.cboxExit.Location = new System.Drawing.Point(213, 0);
            this.cboxExit.Name = "cboxExit";
            this.cboxExit.Size = new System.Drawing.Size(45, 28);
            this.cboxExit.TabIndex = 6;
            // 
            // dControlPanel
            // 
            this.dControlPanel.DockIndicatorTransparencyValue = 0.6D;
            this.dControlPanel.TargetControl = this.label1;
            this.dControlPanel.UseTransparentDrag = true;
            // 
            // gboxMain
            // 
            this.gboxMain.Controls.Add(this.label3);
            this.gboxMain.Controls.Add(this.pbarProgress);
            this.gboxMain.Controls.Add(this.label4);
            this.gboxMain.Controls.Add(this.btnRetrieveWatermark);
            this.gboxMain.Controls.Add(this.txtWatermark);
            this.gboxMain.Controls.Add(this.label2);
            this.gboxMain.Controls.Add(this.btnAddWatermarkToDirectory);
            this.gboxMain.Controls.Add(this.btnAddWatermark);
            this.gboxMain.FillColor = System.Drawing.Color.FromArgb(((int)(((byte)(223)))), ((int)(((byte)(227)))), ((int)(((byte)(238)))));
            this.gboxMain.Font = new System.Drawing.Font("Segoe UI", 9F);
            this.gboxMain.ForeColor = System.Drawing.Color.FromArgb(((int)(((byte)(125)))), ((int)(((byte)(137)))), ((int)(((byte)(149)))));
            this.gboxMain.Location = new System.Drawing.Point(0, 28);
            this.gboxMain.Name = "gboxMain";
            this.gboxMain.Size = new System.Drawing.Size(258, 151);
            this.gboxMain.TabIndex = 8;
            // 
            // label3
            // 
            this.label3.AutoSize = true;
            this.label3.BackColor = System.Drawing.Color.FromArgb(((int)(((byte)(223)))), ((int)(((byte)(227)))), ((int)(((byte)(238)))));
            this.label3.Font = new System.Drawing.Font("Segoe UI Light", 12F, System.Drawing.FontStyle.Regular, System.Drawing.GraphicsUnit.Point, ((byte)(0)));
            this.label3.ForeColor = System.Drawing.Color.FromArgb(((int)(((byte)(82)))), ((int)(((byte)(85)))), ((int)(((byte)(93)))));
            this.label3.Location = new System.Drawing.Point(3, 66);
            this.label3.Name = "label3";
            this.label3.Size = new System.Drawing.Size(135, 21);
            this.label3.TabIndex = 21;
            this.label3.Text = "Watermark Extract:";
            // 
            // pbarProgress
            // 
            this.pbarProgress.FillColor = System.Drawing.Color.FromArgb(((int)(((byte)(178)))), ((int)(((byte)(181)))), ((int)(((byte)(190)))));
            this.pbarProgress.Location = new System.Drawing.Point(0, 105);
            this.pbarProgress.Name = "pbarProgress";
            this.pbarProgress.Size = new System.Drawing.Size(258, 22);
            this.pbarProgress.TabIndex = 20;
            this.pbarProgress.TextRenderingHint = System.Drawing.Text.TextRenderingHint.SystemDefault;
            // 
            // label4
            // 
            this.label4.AutoSize = true;
            this.label4.BackColor = System.Drawing.Color.FromArgb(((int)(((byte)(223)))), ((int)(((byte)(227)))), ((int)(((byte)(238)))));
            this.label4.Font = new System.Drawing.Font("Segoe UI Light", 12F, System.Drawing.FontStyle.Regular, System.Drawing.GraphicsUnit.Point, ((byte)(0)));
            this.label4.ForeColor = System.Drawing.Color.FromArgb(((int)(((byte)(82)))), ((int)(((byte)(85)))), ((int)(((byte)(93)))));
            this.label4.Location = new System.Drawing.Point(3, 31);
            this.label4.Name = "label4";
            this.label4.Size = new System.Drawing.Size(70, 21);
            this.label4.TabIndex = 19;
            this.label4.Text = "Apply To:";
            // 
            // btnRetrieveWatermark
            // 
            this.btnRetrieveWatermark.Animated = true;
            this.btnRetrieveWatermark.DisabledState.BorderColor = System.Drawing.Color.DarkGray;
            this.btnRetrieveWatermark.DisabledState.CustomBorderColor = System.Drawing.Color.DarkGray;
            this.btnRetrieveWatermark.DisabledState.FillColor = System.Drawing.Color.FromArgb(((int)(((byte)(169)))), ((int)(((byte)(169)))), ((int)(((byte)(169)))));
            this.btnRetrieveWatermark.DisabledState.ForeColor = System.Drawing.Color.FromArgb(((int)(((byte)(141)))), ((int)(((byte)(141)))), ((int)(((byte)(141)))));
            this.btnRetrieveWatermark.FillColor = System.Drawing.Color.FromArgb(((int)(((byte)(139)))), ((int)(((byte)(157)))), ((int)(((byte)(195)))));
            this.btnRetrieveWatermark.Font = new System.Drawing.Font("Segoe UI", 9F);
            this.btnRetrieveWatermark.ForeColor = System.Drawing.Color.White;
            this.btnRetrieveWatermark.HoverState.FillColor = System.Drawing.Color.FromArgb(((int)(((byte)(59)))), ((int)(((byte)(89)))), ((int)(((byte)(152)))));
            this.btnRetrieveWatermark.Location = new System.Drawing.Point(144, 59);
            this.btnRetrieveWatermark.Name = "btnRetrieveWatermark";
            this.btnRetrieveWatermark.Size = new System.Drawing.Size(108, 37);
            this.btnRetrieveWatermark.TabIndex = 16;
            this.btnRetrieveWatermark.Text = "Retrieve Watermark";
            this.btnRetrieveWatermark.Click += new System.EventHandler(this.btnRetrieveWatermark_Click);
            // 
            // txtWatermark
            // 
            this.txtWatermark.Animated = true;
            this.txtWatermark.Cursor = System.Windows.Forms.Cursors.IBeam;
            this.txtWatermark.DefaultText = "";
            this.txtWatermark.DisabledState.BorderColor = System.Drawing.Color.FromArgb(((int)(((byte)(208)))), ((int)(((byte)(208)))), ((int)(((byte)(208)))));
            this.txtWatermark.DisabledState.FillColor = System.Drawing.Color.FromArgb(((int)(((byte)(226)))), ((int)(((byte)(226)))), ((int)(((byte)(226)))));
            this.txtWatermark.DisabledState.ForeColor = System.Drawing.Color.FromArgb(((int)(((byte)(138)))), ((int)(((byte)(138)))), ((int)(((byte)(138)))));
            this.txtWatermark.DisabledState.PlaceholderForeColor = System.Drawing.Color.FromArgb(((int)(((byte)(138)))), ((int)(((byte)(138)))), ((int)(((byte)(138)))));
            this.txtWatermark.FillColor = System.Drawing.Color.FromArgb(((int)(((byte)(200)))), ((int)(((byte)(204)))), ((int)(((byte)(214)))));
            this.txtWatermark.FocusedState.BorderColor = System.Drawing.Color.FromArgb(((int)(((byte)(94)))), ((int)(((byte)(148)))), ((int)(((byte)(255)))));
            this.txtWatermark.Font = new System.Drawing.Font("Segoe UI", 9F);
            this.txtWatermark.ForeColor = System.Drawing.Color.FromArgb(((int)(((byte)(82)))), ((int)(((byte)(85)))), ((int)(((byte)(93)))));
            this.txtWatermark.HoverState.BorderColor = System.Drawing.Color.FromArgb(((int)(((byte)(59)))), ((int)(((byte)(89)))), ((int)(((byte)(152)))));
            this.txtWatermark.Location = new System.Drawing.Point(113, 4);
            this.txtWatermark.Name = "txtWatermark";
            this.txtWatermark.PasswordChar = '\0';
            this.txtWatermark.PlaceholderForeColor = System.Drawing.Color.FromArgb(((int)(((byte)(109)))), ((int)(((byte)(114)))), ((int)(((byte)(124)))));
            this.txtWatermark.PlaceholderText = "Placeholder";
            this.txtWatermark.SelectedText = "";
            this.txtWatermark.ShadowDecoration.BorderRadius = 1;
            this.txtWatermark.Size = new System.Drawing.Size(139, 21);
            this.txtWatermark.TabIndex = 14;
            this.txtWatermark.TextAlign = System.Windows.Forms.HorizontalAlignment.Center;
            // 
            // label2
            // 
            this.label2.AutoSize = true;
            this.label2.BackColor = System.Drawing.Color.FromArgb(((int)(((byte)(223)))), ((int)(((byte)(227)))), ((int)(((byte)(238)))));
            this.label2.Font = new System.Drawing.Font("Segoe UI Light", 12F, System.Drawing.FontStyle.Regular, System.Drawing.GraphicsUnit.Point, ((byte)(0)));
            this.label2.ForeColor = System.Drawing.Color.FromArgb(((int)(((byte)(82)))), ((int)(((byte)(85)))), ((int)(((byte)(93)))));
            this.label2.Location = new System.Drawing.Point(3, 3);
            this.label2.Name = "label2";
            this.label2.Size = new System.Drawing.Size(115, 21);
            this.label2.TabIndex = 12;
            this.label2.Text = "Watermark Text:";
            // 
            // btnAddWatermarkToDirectory
            // 
            this.btnAddWatermarkToDirectory.Animated = true;
            this.btnAddWatermarkToDirectory.DisabledState.BorderColor = System.Drawing.Color.DarkGray;
            this.btnAddWatermarkToDirectory.DisabledState.CustomBorderColor = System.Drawing.Color.DarkGray;
            this.btnAddWatermarkToDirectory.DisabledState.FillColor = System.Drawing.Color.FromArgb(((int)(((byte)(169)))), ((int)(((byte)(169)))), ((int)(((byte)(169)))));
            this.btnAddWatermarkToDirectory.DisabledState.ForeColor = System.Drawing.Color.FromArgb(((int)(((byte)(141)))), ((int)(((byte)(141)))), ((int)(((byte)(141)))));
            this.btnAddWatermarkToDirectory.FillColor = System.Drawing.Color.FromArgb(((int)(((byte)(139)))), ((int)(((byte)(157)))), ((int)(((byte)(195)))));
            this.btnAddWatermarkToDirectory.Font = new System.Drawing.Font("Segoe UI", 9F);
            this.btnAddWatermarkToDirectory.ForeColor = System.Drawing.Color.White;
            this.btnAddWatermarkToDirectory.HoverState.FillColor = System.Drawing.Color.FromArgb(((int)(((byte)(59)))), ((int)(((byte)(89)))), ((int)(((byte)(152)))));
            this.btnAddWatermarkToDirectory.Location = new System.Drawing.Point(175, 31);
            this.btnAddWatermarkToDirectory.Name = "btnAddWatermarkToDirectory";
            this.btnAddWatermarkToDirectory.Size = new System.Drawing.Size(77, 24);
            this.btnAddWatermarkToDirectory.TabIndex = 11;
            this.btnAddWatermarkToDirectory.Text = "Directory";
            this.btnAddWatermarkToDirectory.Click += new System.EventHandler(this.btnAddWatermarkToDirectory_Click);
            // 
            // btnAddWatermark
            // 
            this.btnAddWatermark.Animated = true;
            this.btnAddWatermark.DisabledState.BorderColor = System.Drawing.Color.DarkGray;
            this.btnAddWatermark.DisabledState.CustomBorderColor = System.Drawing.Color.DarkGray;
            this.btnAddWatermark.DisabledState.FillColor = System.Drawing.Color.FromArgb(((int)(((byte)(169)))), ((int)(((byte)(169)))), ((int)(((byte)(169)))));
            this.btnAddWatermark.DisabledState.ForeColor = System.Drawing.Color.FromArgb(((int)(((byte)(141)))), ((int)(((byte)(141)))), ((int)(((byte)(141)))));
            this.btnAddWatermark.FillColor = System.Drawing.Color.FromArgb(((int)(((byte)(139)))), ((int)(((byte)(157)))), ((int)(((byte)(195)))));
            this.btnAddWatermark.Font = new System.Drawing.Font("Segoe UI", 9F);
            this.btnAddWatermark.ForeColor = System.Drawing.Color.White;
            this.btnAddWatermark.HoverState.FillColor = System.Drawing.Color.FromArgb(((int)(((byte)(59)))), ((int)(((byte)(89)))), ((int)(((byte)(152)))));
            this.btnAddWatermark.Location = new System.Drawing.Point(73, 31);
            this.btnAddWatermark.Name = "btnAddWatermark";
            this.btnAddWatermark.Size = new System.Drawing.Size(96, 24);
            this.btnAddWatermark.TabIndex = 9;
            this.btnAddWatermark.Text = "Single Image";
            this.btnAddWatermark.Click += new System.EventHandler(this.btnAddWatermark_Click);
            // 
            // pnlBottom
            // 
            this.pnlBottom.Controls.Add(this.picGithub);
            this.pnlBottom.Dock = System.Windows.Forms.DockStyle.Bottom;
            this.pnlBottom.FillColor = System.Drawing.Color.FromArgb(((int)(((byte)(139)))), ((int)(((byte)(157)))), ((int)(((byte)(195)))));
            this.pnlBottom.Location = new System.Drawing.Point(0, 156);
            this.pnlBottom.Name = "pnlBottom";
            this.pnlBottom.Size = new System.Drawing.Size(258, 28);
            this.pnlBottom.TabIndex = 9;
            // 
            // picGithub
            // 
            this.picGithub.BackColor = System.Drawing.Color.Transparent;
            this.picGithub.Dock = System.Windows.Forms.DockStyle.Right;
            this.picGithub.Image = ((System.Drawing.Image)(resources.GetObject("picGithub.Image")));
            this.picGithub.ImageRotate = 0F;
            this.picGithub.Location = new System.Drawing.Point(238, 0);
            this.picGithub.Name = "picGithub";
            this.picGithub.Size = new System.Drawing.Size(20, 28);
            this.picGithub.SizeMode = System.Windows.Forms.PictureBoxSizeMode.Zoom;
            this.picGithub.TabIndex = 9;
            this.picGithub.TabStop = false;
            this.picGithub.UseTransparentBackground = true;
            this.picGithub.Click += new System.EventHandler(this.picGithub_Click);
            // 
            // MainForm
            // 
            this.AutoScaleDimensions = new System.Drawing.SizeF(6F, 13F);
            this.AutoScaleMode = System.Windows.Forms.AutoScaleMode.Font;
            this.BackColor = System.Drawing.SystemColors.ControlDark;
            this.ClientSize = new System.Drawing.Size(258, 184);
            this.Controls.Add(this.pnlBottom);
            this.Controls.Add(this.gboxMain);
            this.Controls.Add(this.pnlHeader);
            this.FormBorderStyle = System.Windows.Forms.FormBorderStyle.None;
            this.Icon = ((System.Drawing.Icon)(resources.GetObject("$this.Icon")));
            this.MaximizeBox = false;
            this.MaximumSize = new System.Drawing.Size(258, 184);
            this.MinimizeBox = false;
            this.MinimumSize = new System.Drawing.Size(258, 184);
            this.Name = "MainForm";
            this.Text = "Form1";
            this.pnlHeader.ResumeLayout(false);
            this.pnlHeader.PerformLayout();
            ((System.ComponentModel.ISupportInitialize)(this.picLogo)).EndInit();
            this.gboxMain.ResumeLayout(false);
            this.gboxMain.PerformLayout();
            this.pnlBottom.ResumeLayout(false);
            ((System.ComponentModel.ISupportInitialize)(this.picGithub)).EndInit();
            this.ResumeLayout(false);

        }

        #endregion
        private Guna.UI2.WinForms.Guna2BorderlessForm borderlessForm;
        private Guna.UI2.WinForms.Guna2Panel pnlHeader;
        private Guna.UI2.WinForms.Guna2ControlBox cboxExit;
        private Guna.UI2.WinForms.Guna2DragControl dControlPanel;
        private Guna.UI2.WinForms.Guna2ControlBox cboxMinimize;
        private System.Windows.Forms.Label label1;
        private Guna.UI2.WinForms.Guna2PictureBox picLogo;
        private Guna.UI2.WinForms.Guna2GroupBox gboxMain;
        private Guna.UI2.WinForms.Guna2Button btnAddWatermark;
        private Guna.UI2.WinForms.Guna2TextBox txtWatermark;
        private System.Windows.Forms.Label label2;
        private Guna.UI2.WinForms.Guna2Button btnAddWatermarkToDirectory;
        private Guna.UI2.WinForms.Guna2Button btnRetrieveWatermark;
        private Guna.UI2.WinForms.Guna2ProgressBar pbarProgress;
        private System.Windows.Forms.Label label4;
        private Guna.UI2.WinForms.Guna2Panel pnlBottom;
        private System.Windows.Forms.Label label3;
        private Guna.UI2.WinForms.Guna2PictureBox picGithub;
    }
}

