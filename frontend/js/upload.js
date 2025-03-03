document.addEventListener('DOMContentLoaded', function() {
    // API base URL - this will be your API Gateway URL when deployed
    const API_BASE_URL = 'https://pvhymfafqoym6f7uj4wj4dzsh40bprml.lambda-url.us-east-1.on.aws';
    
    const uploadForm = document.getElementById('uploadForm');
    const uploadButton = document.querySelector('#uploadForm button[type="submit"]');
    const uploadBtnText = document.getElementById('uploadBtnText');
    const uploadSpinner = document.getElementById('uploadSpinner');
    const progressContainer = document.createElement('div');
    progressContainer.className = 'mt-3';
    uploadForm.appendChild(progressContainer);
    
    uploadForm.addEventListener('submit', async function(e) {
        e.preventDefault();
        
        // Show loading state
        uploadButton.disabled = true;
        uploadBtnText.textContent = 'Uploading...';
        uploadSpinner.classList.remove('d-none');
        
        // Reset progress container
        progressContainer.innerHTML = '';
        
        // Get form data
        const dicomFiles = document.getElementById('dicomFolder').files;
        if (!dicomFiles || dicomFiles.length === 0) {
            alert('Please select a folder with DICOM files');
            resetUploadButton();
            return;
        }

        console.log(`Processing ${dicomFiles.length} files...`);
        
        // Create progress info
        const progressInfo = document.createElement('div');
        progressInfo.className = 'alert alert-info';
        progressInfo.innerHTML = `<p>Processing ${dicomFiles.length} DICOM files...</p>`;
        progressContainer.appendChild(progressInfo);
        
        // Extract common form values
        const commonData = {
            title: document.getElementById('title').value,
            description: document.getElementById('description').value,
            anatomy: document.getElementById('anatomy').value,
            diagnosis: document.getElementById('diagnosis').value,
            findings: document.getElementById('findings').value,
            tags: document.getElementById('tags').value.split(',')
                .map(tag => tag.trim())
                .filter(tag => tag)
        };
        
        try {
            // Process the first file to create the case
            progressInfo.innerHTML += `<p>Processing file 1 of ${dicomFiles.length}...</p>`;
            
            // Read first file
            const firstFile = dicomFiles[0];
            const firstFileBase64 = await readFileAsBase64(firstFile);
            
            // Create case with first file
            const caseData = {
                ...commonData,
                dicomFile: firstFileBase64
            };
            
            // Submit to create new case
            const response = await fetch(`${API_BASE_URL}/api/cases`, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json'
                },
                body: JSON.stringify(caseData)
            });
            
            if (!response.ok) {
                throw new Error(`Failed to create case (status ${response.status})`);
            }
            
            const data = await response.json();
            if (!data.success) {
                throw new Error(data.error || 'Failed to create case');
            }
            
            const caseId = data.data.case_id;
            progressInfo.innerHTML += `<p>Case created successfully with ID: ${caseId}</p>`;
            
            // Process remaining files (if any)
            let successCount = 1; // First file already succeeded
            let errorCount = 0;
            
            if (dicomFiles.length > 1) {
                progressInfo.innerHTML += `<p>Uploading remaining ${dicomFiles.length - 1} files to case...</p>`;
                
                // Process each additional file
                for (let i = 1; i < dicomFiles.length; i++) {
                    try {
                        progressInfo.innerHTML += `<p>Processing file ${i+1} of ${dicomFiles.length}...</p>`;
                        const file = dicomFiles[i];
                        const fileBase64 = await readFileAsBase64(file);
                        
                        // Submit additional file to the new endpoint
                        const addFileResponse = await fetch(`${API_BASE_URL}/api/cases/${caseId}/images`, {
                            method: 'POST',
                            headers: {
                                'Content-Type': 'application/json'
                            },
                            body: JSON.stringify({
                                dicomFile: fileBase64
                            })
                        });
                        
                        if (!addFileResponse.ok) {
                            throw new Error(`Failed to add file ${i+1} (status ${addFileResponse.status})`);
                        }
                        
                        const addFileData = await addFileResponse.json();
                        if (!addFileData.success) {
                            throw new Error(addFileData.error || `Failed to add file ${i+1}`);
                        }
                        
                        successCount++;
                        progressInfo.innerHTML += `<p>File ${i+1} uploaded successfully.</p>`;
                        
                    } catch (fileError) {
                        console.error(`Error processing file ${i+1}:`, fileError);
                        errorCount++;
                        progressInfo.innerHTML += `<p class="text-danger">Error uploading file ${i+1}: ${fileError.message}</p>`;
                    }
                }
            }
            
            // Show summary
            const summaryClass = errorCount > 0 ? 'alert-warning' : 'alert-success';
            const summaryAlert = document.createElement('div');
            summaryAlert.className = `alert ${summaryClass} mt-3`;
            summaryAlert.innerHTML = `
                <h5>Upload Complete</h5>
                <p>${successCount} of ${dicomFiles.length} files uploaded successfully.</p>
                ${errorCount > 0 ? `<p>${errorCount} files failed to upload.</p>` : ''}
                <a href="case.html?id=${caseId}" class="btn btn-primary">View Case</a>
            `;
            progressContainer.appendChild(summaryAlert);
            
            resetUploadButton();
            
        } catch (error) {
            console.error('Upload failed:', error);
            
            const errorAlert = document.createElement('div');
            errorAlert.className = 'alert alert-danger mt-3';
            errorAlert.innerHTML = `
                <h5>Upload Failed</h5>
                <p>${error.message}</p>
            `;
            progressContainer.appendChild(errorAlert);
            
            resetUploadButton();
        }
    });
    
    function resetUploadButton() {
        uploadButton.disabled = false;
        uploadBtnText.textContent = 'Upload Teaching File';
        uploadSpinner.classList.add('d-none');
    }
    
    // Helper function to read a file as base64
    function readFileAsBase64(file) {
        return new Promise((resolve, reject) => {
            const reader = new FileReader();
            reader.onload = event => {
                const base64Data = event.target.result.split(',')[1]; // Remove data URL prefix
                resolve(base64Data);
            };
            reader.onerror = () => reject(new Error('Failed to read file'));
            reader.readAsDataURL(file);
        });
    }
});