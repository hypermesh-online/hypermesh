// Interactive Demo Script for Hello World Example

class DemoRunner {
    constructor() {
        this.demoOutput = document.getElementById('demo-output');
        this.demoCursor = document.getElementById('demo-cursor');
        this.currentStep = 0;
        this.isRunning = false;
        this.stepMode = false;
        
        this.demoSteps = [
            {
                text: "🚀 Hello World - Nexus Deployment Starting...",
                type: "info",
                delay: 1000
            },
            {
                text: "✅ System requirements validated",
                type: "success",
                delay: 800
            },
            {
                text: "  Rust found: rustc 1.88.0",
                type: "normal",
                delay: 500
            },
            {
                text: "  Memory: 62GB available",
                type: "normal",
                delay: 500
            },
            {
                text: "  Ports 8080-8082 available",
                type: "normal",
                delay: 500
            },
            {
                text: "📦 Building Nexus components...",
                type: "info",
                delay: 2000
            },
            {
                text: "✅ Build completed successfully",
                type: "success",
                delay: 1000
            },
            {
                text: "🚀 Deploying 3-node local cluster...",
                type: "info",
                delay: 1500
            },
            {
                text: "  Node 1: Starting on port 8080...",
                type: "normal",
                delay: 800
            },
            {
                text: "  Node 2: Starting on port 8081...",
                type: "normal",
                delay: 800
            },
            {
                text: "  Node 3: Starting on port 8082...",
                type: "normal",
                delay: 800
            },
            {
                text: "✅ All nodes started successfully",
                type: "success",
                delay: 1000
            },
            {
                text: "🔍 Running health checks...",
                type: "info",
                delay: 1500
            },
            {
                text: "  ✅ Transport layer: QUIC connections established",
                type: "success",
                delay: 700
            },
            {
                text: "  ✅ Consensus engine: Byzantine agreement active",
                type: "success",
                delay: 700
            },
            {
                text: "  ✅ Networking: Service discovery operational",
                type: "success",
                delay: 700
            },
            {
                text: "  ✅ eBPF integration: Kernel modules loaded",
                type: "success",
                delay: 700
            },
            {
                text: "📊 Collecting real-time metrics (30s)...",
                type: "info",
                delay: 2000
            },
            {
                text: "  Consensus latency: 8.2ms (p99)",
                type: "normal",
                delay: 1000
            },
            {
                text: "  Network throughput: 1.2 GB/s",
                type: "normal",
                delay: 1000
            },
            {
                text: "  Memory usage: 47MB total",
                type: "normal",
                delay: 1000
            },
            {
                text: "  Byzantine fault tolerance: Active (f=1)",
                type: "normal",
                delay: 1000
            },
            {
                text: "✅ Metrics collection completed",
                type: "success",
                delay: 1000
            },
            {
                text: "🎉 Hello World cluster is ready!",
                type: "info",
                delay: 500
            },
            {
                text: "",
                type: "normal",
                delay: 300
            },
            {
                text: "Cluster Information:",
                type: "normal",
                delay: 300
            },
            {
                text: "  Node 1: http://localhost:8080 ✅",
                type: "normal",
                delay: 400
            },
            {
                text: "  Node 2: http://localhost:8081 ✅",
                type: "normal",
                delay: 400
            },
            {
                text: "  Node 3: http://localhost:8082 ✅",
                type: "normal",
                delay: 400
            },
            {
                text: "",
                type: "normal",
                delay: 300
            },
            {
                text: "Next: Try './status.sh --watch' for live monitoring",
                type: "info",
                delay: 1000
            }
        ];
        
        this.initializeControls();
    }
    
    initializeControls() {
        const startButton = document.getElementById('start-demo');
        const stepButton = document.getElementById('step-mode');
        const resetButton = document.getElementById('reset-demo');
        
        if (startButton) {
            startButton.addEventListener('click', () => this.startDemo());
        }
        
        if (stepButton) {
            stepButton.addEventListener('click', () => this.toggleStepMode());
        }
        
        if (resetButton) {
            resetButton.addEventListener('click', () => this.resetDemo());
        }
    }
    
    async startDemo() {
        if (this.isRunning) return;
        
        this.isRunning = true;
        const startButton = document.getElementById('start-demo');
        if (startButton) {
            startButton.textContent = 'Running...';
            startButton.disabled = true;
        }
        
        // Hide cursor during demo
        this.demoCursor.style.display = 'none';
        
        for (let i = this.currentStep; i < this.demoSteps.length; i++) {
            if (!this.isRunning) break;
            
            await this.executeStep(i);
            this.currentStep = i + 1;
            
            if (this.stepMode) {
                await this.waitForUser();
            }
        }
        
        this.completeDemo();
    }
    
    async executeStep(stepIndex) {
        const step = this.demoSteps[stepIndex];
        const line = this.createTerminalLine(step.text, step.type);
        
        // Add line to terminal
        this.demoOutput.insertBefore(line, this.demoCursor);
        
        // Animate line appearance
        setTimeout(() => {
            line.classList.add('visible');
        }, 50);
        
        // Scroll to bottom
        this.scrollToBottom();
        
        // Wait for step delay
        await this.sleep(step.delay);
    }
    
    createTerminalLine(text, type = 'normal') {
        const line = document.createElement('div');
        line.className = `terminal-line ${type}`;
        line.textContent = text;
        return line;
    }
    
    async waitForUser() {
        return new Promise((resolve) => {
            const continueButton = document.createElement('button');
            continueButton.textContent = 'Continue';
            continueButton.className = 'btn btn-primary';
            continueButton.style.margin = '1rem 0';
            
            continueButton.addEventListener('click', () => {
                continueButton.remove();
                resolve();
            });
            
            this.demoOutput.insertBefore(continueButton, this.demoCursor);
        });
    }
    
    toggleStepMode() {
        this.stepMode = !this.stepMode;
        const stepButton = document.getElementById('step-mode');
        if (stepButton) {
            stepButton.textContent = this.stepMode ? 'Continuous Mode' : 'Step by Step';
            stepButton.className = this.stepMode ? 'btn btn-secondary' : 'btn btn-secondary';
        }
    }
    
    resetDemo() {
        this.isRunning = false;
        this.currentStep = 0;
        
        // Clear all demo output except the prompt line
        const lines = this.demoOutput.querySelectorAll('.terminal-line:not(.prompt-line)');
        lines.forEach(line => line.remove());
        
        // Remove any continue buttons
        const buttons = this.demoOutput.querySelectorAll('button');
        buttons.forEach(button => button.remove());
        
        // Reset controls
        const startButton = document.getElementById('start-demo');
        if (startButton) {
            startButton.textContent = 'Run Demo';
            startButton.disabled = false;
        }
        
        // Show cursor
        this.demoCursor.style.display = 'inline-block';
    }
    
    completeDemo() {
        this.isRunning = false;
        
        const startButton = document.getElementById('start-demo');
        if (startButton) {
            startButton.textContent = 'Demo Complete!';
            startButton.disabled = false;
        }
        
        // Show cursor
        this.demoCursor.style.display = 'inline-block';
        
        // Add completion message
        setTimeout(() => {
            const completionLine = this.createTerminalLine('Demo completed! Try the manual steps above or reset to run again.', 'info');
            this.demoOutput.insertBefore(completionLine, this.demoCursor);
            completionLine.classList.add('visible');
            this.scrollToBottom();
        }, 1000);
    }
    
    scrollToBottom() {
        this.demoOutput.scrollTop = this.demoOutput.scrollHeight;
    }
    
    sleep(ms) {
        return new Promise(resolve => setTimeout(resolve, ms));
    }
}

// Enhanced copy code functionality
function copyCode(button) {
    const codeBlock = button.parentNode.querySelector('code');
    const text = codeBlock.textContent;
    
    navigator.clipboard.writeText(text).then(() => {
        const originalText = button.textContent;
        const originalBg = button.style.background;
        
        button.textContent = 'Copied!';
        button.style.background = '#10b981';
        button.style.transform = 'scale(0.95)';
        
        setTimeout(() => {
            button.textContent = originalText;
            button.style.background = originalBg || '#6366f1';
            button.style.transform = 'scale(1)';
        }, 2000);
    }).catch(() => {
        // Fallback for browsers without clipboard API
        const textArea = document.createElement('textarea');
        textArea.value = text;
        textArea.style.position = 'fixed';
        textArea.style.left = '-999999px';
        textArea.style.top = '-999999px';
        document.body.appendChild(textArea);
        textArea.focus();
        textArea.select();
        
        try {
            document.execCommand('copy');
            button.textContent = 'Copied!';
            button.style.background = '#10b981';
            
            setTimeout(() => {
                button.textContent = 'Copy';
                button.style.background = '#6366f1';
            }, 2000);
        } catch (err) {
            console.error('Copy failed:', err);
        }
        
        document.body.removeChild(textArea);
    });
}

// Add realistic terminal typing effect to static code blocks
function addTypingEffect() {
    const codeBlocks = document.querySelectorAll('.code-block code');
    
    codeBlocks.forEach((code, index) => {
        // Add a subtle delay to stagger animations
        setTimeout(() => {
            code.style.opacity = '0';
            code.style.transform = 'translateY(10px)';
            code.style.transition = 'all 0.5s ease';
            
            setTimeout(() => {
                code.style.opacity = '1';
                code.style.transform = 'translateY(0)';
            }, 100);
        }, index * 200);
    });
}

// Initialize everything when DOM is loaded
document.addEventListener('DOMContentLoaded', () => {
    // Initialize the demo runner
    if (document.getElementById('demo-output')) {
        new DemoRunner();
    }
    
    // Add typing effects to code blocks
    addTypingEffect();
    
    // Add smooth scrolling for anchor links
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
    
    // Add intersection observer for animations
    const observerOptions = {
        threshold: 0.1,
        rootMargin: '0px 0px -50px 0px'
    };
    
    const observer = new IntersectionObserver((entries) => {
        entries.forEach(entry => {
            if (entry.isIntersecting) {
                entry.target.style.opacity = '1';
                entry.target.style.transform = 'translateY(0)';
            }
        });
    }, observerOptions);
    
    // Observe sections for animations
    document.querySelectorAll('.overview-item, .script-card, .objective-card, .next-example-card').forEach(el => {
        el.style.opacity = '0';
        el.style.transform = 'translateY(20px)';
        el.style.transition = 'all 0.6s ease';
        observer.observe(el);
    });
});

// Add some interactive features
function highlightCommand(element) {
    element.style.background = 'rgba(99, 102, 241, 0.1)';
    element.style.borderColor = '#6366f1';
    
    setTimeout(() => {
        element.style.background = '';
        element.style.borderColor = '';
    }, 2000);
}

// Make code blocks clickable to highlight
document.addEventListener('DOMContentLoaded', () => {
    document.querySelectorAll('.code-block').forEach(block => {
        block.addEventListener('click', (e) => {
            if (e.target.tagName !== 'BUTTON') {
                highlightCommand(block);
            }
        });
        
        // Add hover effect
        block.addEventListener('mouseenter', () => {
            block.style.transform = 'translateY(-2px)';
            block.style.boxShadow = '0 4px 12px rgba(0, 0, 0, 0.15)';
        });
        
        block.addEventListener('mouseleave', () => {
            block.style.transform = 'translateY(0)';
            block.style.boxShadow = '';
        });
    });
});