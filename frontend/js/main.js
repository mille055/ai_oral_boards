'use strict';
console.log("Main.js loaded successfully");

document.addEventListener('DOMContentLoaded', function() {
    const caseContainer = document.getElementById('caseContainer');
    const noCasesMessage = document.getElementById('noCasesMessage');
    const loadingIndicator = document.getElementById('loadingIndicator');
    const modalityFilter = document.getElementById('modalityFilter');
    const anatomyFilter = document.getElementById('anatomyFilter');
    const applyFiltersBtn = document.getElementById('applyFilters');
    
    // API base URL - this should match your Lambda URL
    const API_BASE_URL = 'https://pvhymfafqoym6f7uj4wj4dzsh40bprml.lambda-url.us-east-1.on.aws';
    
    // Fetch and display all cases
    function fetchCases() {
        // Show loading indicator
        loadingIndicator.classList.remove('d-none');
        caseContainer.innerHTML = '';
        
        fetch(`${API_BASE_URL}/api/cases`)
            .then(response => response.json())
            .then(responseData => {
                // Hide loading indicator
                loadingIndicator.classList.add('d-none');
                
                // Check response structure
                if (!responseData.success) {
                    throw new Error(responseData.error || 'Failed to fetch cases');
                }
                
                const cases = responseData.data;
                
                if (cases.length === 0) {
                    noCasesMessage.classList.remove('d-none');
                    return;
                }
                
                noCasesMessage.classList.add('d-none');
                displayCases(cases);
            })
            .catch(error => {
                console.error('Error fetching cases:', error);
                loadingIndicator.classList.add('d-none');
                noCasesMessage.textContent = 'Error loading cases. Please try again later.';
                noCasesMessage.classList.remove('d-none');
            });
    }
    
    // Display cases in the UI
    function displayCases(cases) {
        const modality = modalityFilter.value;
        const anatomy = anatomyFilter.value;
        
        // Filter cases if filters are set
        let filteredCases = cases;
        if (modality) {
            filteredCases = filteredCases.filter(c => c.modality === modality);
        }
        if (anatomy) {
            filteredCases = filteredCases.filter(c => c.anatomy === anatomy);
        }
        
        if (filteredCases.length === 0) {
            noCasesMessage.textContent = 'No cases match the selected filters.';
            noCasesMessage.classList.remove('d-none');
            return;
        }
        
        // Sort cases by creation date (newest first)
        filteredCases.sort((a, b) => new Date(b.created_at) - new Date(a.created_at));
        
        // Create case cards
        filteredCases.forEach(caseItem => {
            const imageId = caseItem.image_ids[0]; // Get first image for thumbnail
            const caseCard = document.createElement('div');
            caseCard.className = 'col-md-4 mb-4';
            
            // Use the DICOM API endpoint as the image source
            const imageUrl = `${API_BASE_URL}/api/dicom/${caseItem.case_id}/${imageId}`;
            
            caseCard.innerHTML = `
                <div class="card case-card h-100">
                    <div class="image-container">
                        <img src="${imageUrl}" class="card-img-top" alt="${caseItem.title}" onerror="this.src='images/placeholder.png'">
                    </div>
                    <div class="card-body">
                        <h5 class="card-title">${caseItem.title}</h5>
                        <p class="card-text">${caseItem.description || 'No description provided'}</p>
                        <div class="mb-2">
                            <span class="badge bg-primary">${caseItem.modality || 'Unknown'}</span>
                            <span class="badge bg-success">${caseItem.anatomy || 'Unknown'}</span>
                        </div>
                        <a href="case.html?id=${caseItem.case_id}" class="btn btn-primary">View Case</a>
                    </div>
                </div>
            `;
            
            caseContainer.appendChild(caseCard);
        });
    }
    
    // Apply filters
    applyFiltersBtn.addEventListener('click', function() {
        fetchCases();
    });
    
    // Initial fetch
    fetchCases();
});