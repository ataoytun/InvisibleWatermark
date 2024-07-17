using System;

namespace InvisibleWatermark.Classes
{
    public class CWatermarkException : Exception
    {
        public CWatermarkException(string message) : base(message) { }
        public CWatermarkException(string message, Exception innerException) : base(message, innerException) { }
    }

    public class CImageProcessingException : Exception
    {
        public CImageProcessingException(string message) : base(message) { }
        public CImageProcessingException(string message, Exception innerException) : base(message, innerException) { }
    }
}