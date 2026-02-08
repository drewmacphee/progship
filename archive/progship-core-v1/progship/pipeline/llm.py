"""LLM client abstraction for Ollama-powered text generation."""

from typing import List, Optional
from dataclasses import dataclass

import ollama


@dataclass
class LLMConfig:
    """Configuration for Ollama LLM client."""
    
    model: str = "qwen2.5:7b"  # Ollama model name
    temperature: float = 0.7
    top_p: float = 0.9
    max_tokens: int = 300
    base_url: str = "http://localhost:11434"  # Ollama server URL


class OllamaClient:
    """LLM client using Ollama local inference."""
    
    def __init__(self, config: Optional[LLMConfig] = None):
        """Initialize Ollama client with configuration.
        
        Args:
            config: LLM configuration (uses defaults if None)
        """
        self.config = config or LLMConfig()
        self._client = ollama.Client(host=self.config.base_url)
        
    def generate(self, prompt: str, max_tokens: Optional[int] = None) -> str:
        """Generate text from a single prompt.
        
        Args:
            prompt: Input text prompt
            max_tokens: Override default max_tokens if provided
            
        Returns:
            Generated text response
        """
        options = {
            "temperature": self.config.temperature,
            "top_p": self.config.top_p,
            "num_predict": max_tokens or self.config.max_tokens,
        }
        
        response = self._client.generate(
            model=self.config.model,
            prompt=prompt,
            options=options,
        )
        
        return response["response"]
    
    def batch_generate(self, prompts: List[str], max_tokens: Optional[int] = None) -> List[str]:
        """Generate text from multiple prompts sequentially.
        
        Args:
            prompts: List of input prompts
            max_tokens: Override default max_tokens if provided
            
        Returns:
            List of generated text responses (same order as prompts)
        """
        if not prompts:
            return []
        
        results = []
        for prompt in prompts:
            response = self.generate(prompt, max_tokens)
            results.append(response)
        
        return results
    
    def check_model(self) -> bool:
        """Check if configured model is available.
        
        Returns:
            True if model exists, False otherwise
        """
        try:
            models = self._client.list()
            model_names = [m["name"] for m in models["models"]]
            return self.config.model in model_names
        except Exception:
            return False
    
    def pull_model(self):
        """Download the configured model if not available."""
        print(f"Downloading model: {self.config.model}...")
        self._client.pull(self.config.model)
        print(f"✓ Model {self.config.model} downloaded successfully")


def create_client(
    model: str = "qwen2.5:7b",
    temperature: float = 0.7,
    max_tokens: int = 300,
    base_url: str = "http://localhost:11434",
) -> OllamaClient:
    """Create a configured Ollama client.
    
    Args:
        model: Ollama model name
        temperature: Sampling temperature
        max_tokens: Maximum tokens to generate
        base_url: Ollama server URL
        
    Returns:
        Configured OllamaClient instance
    """
    config = LLMConfig(
        model=model,
        temperature=temperature,
        max_tokens=max_tokens,
        base_url=base_url,
    )
    return OllamaClient(config)
