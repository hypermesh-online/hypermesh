// Hypermesh Nexus Website JavaScript

// Mobile navigation
const hamburger = document.querySelector('.hamburger');
const navMenu = document.querySelector('.nav-menu');

hamburger.addEventListener('click', () => {
    hamburger.classList.toggle('active');
    navMenu.classList.toggle('active');
});

// Smooth scrolling for anchor links
document.querySelectorAll('a[href^="#"]').forEach(anchor => {
    anchor.addEventListener('click', function (e) {
        e.preventDefault();
        const target = document.querySelector(this.getAttribute('href'));
        if (target) {
            target.scrollIntoView({
                behavior: 'smooth',
                block: 'start'
            });
        }
    });
});

// Copy code function
function copyCode(button) {
    const codeBlock = button.parentNode.querySelector('code');
    const text = codeBlock.textContent;
    
    navigator.clipboard.writeText(text).then(() => {
        const originalText = button.textContent;
        button.textContent = 'Copied!';
        button.style.background = '#10b981';
        
        setTimeout(() => {
            button.textContent = originalText;
            button.style.background = '#6366f1';
        }, 2000);
    });
}

// Animated terminal demo
function animateTerminal() {
    const terminalLines = document.querySelectorAll('.terminal-line');
    let delay = 0;
    
    terminalLines.forEach((line, index) => {
        if (index === 0) return; // Skip the command line
        
        setTimeout(() => {
            line.style.opacity = '0';
            line.style.transform = 'translateX(-20px)';
            line.style.transition = 'all 0.5s ease';
            
            setTimeout(() => {
                line.style.opacity = '1';
                line.style.transform = 'translateX(0)';
            }, 100);
        }, delay);
        
        delay += 800;
    });
}

// Animate numbers counting up
function animateCounters() {
    const counters = document.querySelectorAll('.metric-value');
    
    counters.forEach(counter => {
        const target = counter.textContent.replace(/[^\d]/g, '');
        if (!target) return;
        
        const increment = target / 100;
        let current = 0;
        
        const timer = setInterval(() => {
            current += increment;
            counter.textContent = Math.ceil(current) + counter.textContent.replace(/\d/g, '').replace(/^\d+/, '');
            
            if (current >= target) {
                clearInterval(timer);
                counter.textContent = counter.textContent;
            }
        }, 20);
    });
}

// Intersection Observer for animations
const observerOptions = {
    threshold: 0.1,
    rootMargin: '0px 0px -50px 0px'
};

const observer = new IntersectionObserver((entries) => {
    entries.forEach(entry => {
        if (entry.isIntersecting) {
            entry.target.classList.add('fade-in');
            
            // Trigger specific animations
            if (entry.target.querySelector('.terminal')) {
                animateTerminal();
            }
            
            if (entry.target.querySelector('.metric-value')) {
                animateCounters();
            }
        }
    });
}, observerOptions);

// Observe all sections for animation
document.querySelectorAll('.section').forEach(section => {
    observer.observe(section);
});

// Add fade-in animation class
const style = document.createElement('style');
style.textContent = `
    .section {
        opacity: 0;
        transform: translateY(20px);
        transition: all 0.8s ease;
    }
    
    .section.fade-in {
        opacity: 1;
        transform: translateY(0);
    }
    
    .nav-menu.active {
        display: flex;
        position: absolute;
        top: 100%;
        left: 0;
        width: 100%;
        background: white;
        flex-direction: column;
        padding: 1rem 2rem;
        border-top: 1px solid var(--border-color);
        box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
    }
    
    .hamburger.active span:nth-child(1) {
        transform: rotate(45deg) translate(5px, 5px);
    }
    
    .hamburger.active span:nth-child(2) {
        opacity: 0;
    }
    
    .hamburger.active span:nth-child(3) {
        transform: rotate(-45deg) translate(7px, -6px);
    }
`;
document.head.appendChild(style);

// Live demo functionality
function startLiveDemo() {
    const demoButton = document.querySelector('a[href="#demo"]');
    if (demoButton) {
        demoButton.addEventListener('click', (e) => {
            e.preventDefault();
            
            // Create modal for live demo
            const modal = document.createElement('div');
            modal.style.cssText = `
                position: fixed;
                top: 0;
                left: 0;
                width: 100%;
                height: 100%;
                background: rgba(0, 0, 0, 0.8);
                display: flex;
                align-items: center;
                justify-content: center;
                z-index: 2000;
            `;
            
            modal.innerHTML = `
                <div style="
                    background: white;
                    padding: 2rem;
                    border-radius: 1rem;
                    max-width: 90%;
                    max-height: 90%;
                    overflow: auto;
                ">
                    <h2 style="margin-bottom: 1rem;">Live Demo</h2>
                    <p style="margin-bottom: 2rem;">
                        Ready to try Hypermesh Nexus? Choose your demo:
                    </p>
                    <div style="display: grid; gap: 1rem; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));">
                        <a href="examples/hello-world.html" style="
                            display: block;
                            padding: 1rem;
                            background: var(--primary-color);
                            color: white;
                            text-decoration: none;
                            border-radius: 0.5rem;
                            text-align: center;
                        ">Hello World (2 min)</a>
                        <a href="examples/production.html" style="
                            display: block;
                            padding: 1rem;
                            background: var(--secondary-color);
                            color: white;
                            text-decoration: none;
                            border-radius: 0.5rem;
                            text-align: center;
                        ">Production Cluster</a>
                        <a href="docs/quick-start.html" style="
                            display: block;
                            padding: 1rem;
                            background: var(--accent-color);
                            color: white;
                            text-decoration: none;
                            border-radius: 0.5rem;
                            text-align: center;
                        ">Interactive Guide</a>
                    </div>
                    <button onclick="this.parentElement.parentElement.remove()" style="
                        margin-top: 2rem;
                        padding: 0.5rem 1rem;
                        background: #6b7280;
                        color: white;
                        border: none;
                        border-radius: 0.25rem;
                        cursor: pointer;
                        float: right;
                    ">Close</button>
                </div>
            `;
            
            document.body.appendChild(modal);
            
            // Close on backdrop click
            modal.addEventListener('click', (e) => {
                if (e.target === modal) {
                    modal.remove();
                }
            });
        });
    }
}

// Initialize everything when DOM is loaded
document.addEventListener('DOMContentLoaded', () => {
    startLiveDemo();
    
    // Add loading animation
    document.body.style.opacity = '0';
    document.body.style.transition = 'opacity 0.5s ease';
    
    setTimeout(() => {
        document.body.style.opacity = '1';
    }, 100);
});

// Performance metrics animation with realistic values
function updatePerformanceMetrics() {
    const metrics = [
        { selector: '.performance-metric:nth-child(1) .metric-value', value: '< 10ms' },
        { selector: '.performance-metric:nth-child(2) .metric-value', value: '50-90%' },
        { selector: '.performance-metric:nth-child(3) .metric-value', value: '~50MB' },
        { selector: '.performance-metric:nth-child(4) .metric-value', value: '1/3' }
    ];
    
    metrics.forEach((metric, index) => {
        setTimeout(() => {
            const element = document.querySelector(metric.selector);
            if (element) {
                element.style.opacity = '0';
                setTimeout(() => {
                    element.textContent = metric.value;
                    element.style.opacity = '1';
                    element.style.transform = 'scale(1.1)';
                    setTimeout(() => {
                        element.style.transform = 'scale(1)';
                    }, 200);
                }, 200);
            }
        }, index * 300);
    });
}

// Trigger performance animation when section is visible
const performanceSection = document.querySelector('#performance');
if (performanceSection) {
    observer.observe(performanceSection);
}

// Add some interactive features to the cluster diagram
function animateClusterDiagram() {
    const nodes = document.querySelectorAll('.node');
    nodes.forEach((node, index) => {
        setInterval(() => {
            node.style.transform = node.style.transform.includes('scale') ? 
                node.style.transform.replace(/scale\([^)]*\)/, '') : 
                node.style.transform + ' scale(1.1)';
            
            setTimeout(() => {
                node.style.transform = node.style.transform.replace(/scale\([^)]*\)/, '');
            }, 300);
        }, 2000 + index * 400);
    });
}

// Start cluster animation
setTimeout(animateClusterDiagram, 1000);