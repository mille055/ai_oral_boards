document.addEventListener('DOMContentLoaded', function() {
    // API base URL - this will be your API Gateway URL when deployed
    const API_BASE_URL = 'https://pvhymfafqoym6f7uj4wj4dzsh40bprml.lambda-url.us-east-1.on.aws';
    
    const uploadForm = document.getElementById('uploadForm');
    const uploadButton = document.querySelector('#uploadForm button[type="submit"]');
    const uploadBtnText = document.getElementById('uploadBtnText');
    const uploadSpinner = document.getElementById('uploadSpinner');
    
    uploadForm.addEventListener('submit', function(e) {
        e.preventDefault();
        
        // Show loading state
        uploadButton.disabled = true;
        uploadBtnText.textContent = 'Uploading...';
        uploadSpinner.classList.remove('d-none');
        
        // Get form data
        const dicomFiles = document.getElementById('dicomFolder').files;
        if (!dicomFiles || dicomFiles.length === 0) {
            alert('Please select a folder with DICOM files');
            resetUploadButton();
            return;
        }

        // For simplicity, only handle the first file for now
        // In a real application, you would need to handle multiple files
        const file = dicomFiles[0];
        
        // Read file as base64
        const reader = new FileReader();
        reader.onload = function(event) {
            const base64Data = event.target.result.split(',')[1]; // Remove data URL prefix
            
            // Extract form values
            const caseData = {
                title: document.getElementById('title').value,
                description: document.getElementById('description').value,
                anatomy: document.getElementById('anatomy').value,
                diagnosis: document.getElementById('diagnosis').value,
                findings: document.getElementById('findings').value,
                tags: document.getElementById('tags').value.split(',')
                    .map(tag => tag.trim())
                    .filter(tag => tag),
                dicomFile: base64Data
            };

            // Submit the data to the Lambda function
            fetch(`${API_BASE_URL}/api/cases`, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json'
                },
                body: JSON.stringify(caseData)
            })
            .then(response => {
                if (!response.ok) {
                    throw new Error('Network response was not ok');
                }
                return response.json();
            })
            .then(data => {
                if (!data.success) {
                    throw new Error(data.error || 'Failed to upload case');
                }
                
                resetUploadButton();
                alert('Case uploaded successfully!');
        
                // Redirect to the case page
                window.location.href = `case.html?id=${data.data.case_id}`;
            })
            .catch(error => {
                console.error('Upload error:', error);
                resetUploadButton();
                alert('Failed to upload the case: ' + error.message);
            });
        };
        
        reader.onerror = function() {
            console.error('Error reading file');
            resetUploadButton();
            alert('Failed to read the DICOM file');
        };
        
        reader.readAsDataURL(file);
    });
    
    function resetUploadButton() {
        uploadButton.disabled = false;
        uploadBtnText.textContent = 'Upload Teaching File';
        uploadSpinner.classList.add('d-none');
    }
});