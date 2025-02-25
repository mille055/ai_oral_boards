document.addEventListener('DOMContentLoaded', function() {
    // Get case ID from URL
    const urlParams = new URLSearchParams(window.location.search);
    const caseId = urlParams.get('id');
    
    // API base URL - this will be your API Gateway URL when deployed
    const API_BASE_URL = 'https://your-api-gateway-id.execute-api.your-region.amazonaws.com/prod';
    
    if (!caseId) {
        showCaseNotFound();
        return;
    }
    
    // Elements
    const loadingCase = document.getElementById('loadingCase');
    const caseContent = document.getElementById('caseContent');
    const caseNotFound = document.getElementById('caseNotFound');
    
    // Case information elements
    const caseTitle = document.getElementById('caseTitle');
    const caseTitleDetail = document.getElementById('caseTitleDetail');
    const caseDescription = document.getElementById('caseDescription');
    const caseModality = document.getElementById('caseModality');
    const caseAnatomy = document.getElementById('caseAnatomy');
    const caseDiagnosis = document.getElementById('caseDiagnosis');
    const caseFindings = document.getElementById('caseFindings');
    const caseTags = document.getElementById('caseTags');
    const seriesSelector = document.getElementById('seriesSelector');
    const ohifViewer = document.getElementById('ohifViewer');
    const debugInfo = document.getElementById('debugInfo');
    
    // Fetch case details
    fetch(`${API_BASE_URL}/api/cases/${caseId}`)
        .then(response => {
            if (!response.ok) {
                throw new Error('Case not found');
            }
            return response.json();
        })
        .then(response => {
            if (!response.success) {
                throw new Error(response.error || 'Failed to load case');
            }
            
            const caseData = response.data;
            console.log("Case data loaded:", caseData);
            debugInfo.innerHTML += `<p>Case loaded: ${caseData.case_id}</p>`;
            
            // Display case details
            displayCaseDetails(caseData);
            
            // Initialize OHIF viewer with this case
            initializeOhifViewer(caseData);
            
            // Hide loading, show content
            loadingCase.classList.add('d-none');
            caseContent.classList.remove('d-none');
        })
        .catch(error => {
            console.error('Error loading case:', error);
            debugInfo.innerHTML += `<p>Error: ${error.message}</p>`;
            showCaseNotFound();
        });
    
    // Display case details in the UI
    function displayCaseDetails(caseData) {
        // Set page title
        document.title = `${caseData.title} - Radiology Teaching Files`;
        
        // Update UI elements
        caseTitle.textContent = caseData.title;
        caseTitleDetail.textContent = caseData.title;
        caseDescription.textContent = caseData.description || 'No description provided';
        caseModality.textContent = caseData.modality || 'Unknown';
        caseAnatomy.textContent = caseData.anatomy || 'Unknown';
        caseDiagnosis.textContent = caseData.diagnosis || 'No diagnosis provided';
        caseFindings.textContent = caseData.findings || 'No findings documented';
        
        // Display tags
        caseTags.innerHTML = '';
        if (caseData.tags && caseData.tags.length > 0) {
            caseData.tags.forEach(tag => {
                const tagBadge = document.createElement('span');
                tagBadge.className = 'badge bg-secondary me-1 mb-1';
                tagBadge.textContent = tag;
                caseTags.appendChild(tagBadge);
            });
        } else {
            caseTags.textContent = 'No tags';
        }
        
        // Add series buttons if available
        seriesSelector.innerHTML = '';
        if (caseData.series && caseData.series.length > 0) {
            caseData.series.forEach((series, index) => {
                const button = document.createElement('a');
                button.className = 'list-group-item list-group-item-action';
                button.href = '#';
                button.dataset.seriesId = series.series_id;
                
                button.innerHTML = `
                    <div class="d-flex w-100 justify-content-between">
                        <h6 class="mb-1">Series ${index + 1}</h6>
                        <small>${series.image_ids.length} images</small>
                    </div>
                    <p class="mb-1">${series.description || 'Unknown'}</p>
                `;
                
                button.addEventListener('click', (e) => {
                    e.preventDefault();
                    selectSeries(caseData.case_id, series.series_id);
                });
                
                seriesSelector.appendChild(button);
            });
            
            // Set the first series as active
            if (seriesSelector.firstChild) {
                seriesSelector.firstChild.classList.add('active');
            }
        } else {
            // Simple case with no series information
            const button = document.createElement('a');
            button.className = 'list-group-item list-group-item-action active';
            button.href = '#';
            
            button.innerHTML = `
                <div class="d-flex w-100 justify-content-between">
                    <h6 class="mb-1">Primary Series</h6>
                    <small>${caseData.image_ids.length} images</small>
                </div>
                <p class="mb-1">Default Series</p>
            `;
            
            seriesSelector.appendChild(button);
        }
    }
    
    // // Initialize OHIF viewer
    // function initializeOhifViewer(caseData) {
    //     debugInfo.innerHTML += `<p>Initializing OHIF viewer</p>`;
        
    //     // Configure OHIF viewer to use our DICOMweb endpoints
    //     // Note: You'll need to set up a DICOMweb-compatible endpoint in your Lambda
    //     const ohifUrl = `https://ohif-viewer-url.com/viewer?StudyInstanceUIDs=${caseData.case_id}&dicomWebRoot=${API_BASE_URL}/dicomweb`;
    //     debugInfo.innerHTML += `<p>OHIF URL: ${ohifUrl}</p>`;
        
    //     ohifViewer.onload = function() {
    //         debugInfo.innerHTML += `<p>OHIF iframe loaded</p>`;
    //     };
        
    //     ohifViewer.onerror = function(error) {
    //         debugInfo.innerHTML += `<p>OHIF iframe error: ${error}</p>`;
    //         console.error("OHIF iframe failed to load:", error);
    //     };
        
    //     ohifViewer.src = ohifUrl;
    // }
    
    // In case.js - Initialize OHIF viewer
    function initializeOhifViewer(caseData) {
        // Configure OHIF viewer to use our DICOMweb endpoints
        const ohifUrl = `https://viewer.ohif.org/viewer?StudyInstanceUIDs=${caseData.case_id}&dicomWebRoot=${API_BASE_URL}/dicomweb`;
        ohifViewer.src = ohifUrl;
    }

    // Select a specific series
    function selectSeries(caseId, seriesId) {
        console.log("Selecting series:", seriesId, "in case:", caseId);
        debugInfo.innerHTML += `<p>Selected series: ${seriesId}</p>`;
        
        // Update active state in UI
        const seriesButtons = seriesSelector.querySelectorAll('.list-group-item');
        seriesButtons.forEach(button => {
            button.classList.remove('active');
            if (button.getAttribute('data-series-id') === seriesId) {
                button.classList.add('active');
            }
        });
        
        // Try to communicate with the OHIF viewer
        try {
            const frame = document.getElementById('ohifViewer');
            if (frame && frame.contentWindow) {
                console.log("Attempting to communicate with OHIF iframe");
                frame.contentWindow.postMessage({
                    command: 'setActiveSeries',
                    seriesId: seriesId
                }, '*');
            }
        } catch (error) {
            debugInfo.innerHTML += `<p>OHIF communication error: ${error.message}</p>`;
            console.error("Error communicating with OHIF:", error);
        }
    }
    
    // Show case not found message
    function showCaseNotFound() {
        loadingCase.classList.add('d-none');
        caseContent.classList.add('d-none');
        caseNotFound.classList.remove('d-none');
    }
});