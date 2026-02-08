"""
JSON Schema Validator for ProgShip data files.

Validates ship types, style descriptors, facilities, and rooms against their schemas.
"""

import json
from pathlib import Path
from typing import Dict, Any, List, Tuple
import jsonschema
from jsonschema import validate, ValidationError


class SchemaValidator:
    """Validates JSON data against schemas."""
    
    def __init__(self, schema_dir: Path = None, data_dir: Path = None):
        """
        Initialize validator with schema and data directories.
        
        Args:
            schema_dir: Path to directory containing JSON schemas
            data_dir: Path to directory containing JSON data files
        """
        if schema_dir is None:
            schema_dir = Path(__file__).parent.parent.parent / "schemas"
        if data_dir is None:
            data_dir = Path(__file__).parent.parent.parent / "data"
            
        self.schema_dir = Path(schema_dir)
        self.data_dir = Path(data_dir)
        self.schemas: Dict[str, Any] = {}
        
    def load_schema(self, schema_name: str) -> Dict[str, Any]:
        """
        Load a JSON schema from file.
        
        Args:
            schema_name: Name of schema file (without .json extension)
            
        Returns:
            Parsed schema dictionary
        """
        schema_path = self.schema_dir / f"{schema_name}.json"
        with open(schema_path, 'r', encoding='utf-8') as f:
            return json.load(f)
    
    def load_data(self, data_name: str) -> Dict[str, Any]:
        """
        Load a JSON data file.
        
        Args:
            data_name: Name of data file (without .json extension)
            
        Returns:
            Parsed data dictionary
        """
        data_path = self.data_dir / f"{data_name}.json"
        with open(data_path, 'r', encoding='utf-8') as f:
            return json.load(f)
    
    def validate_file(self, data_name: str, schema_name: str) -> Tuple[bool, List[str]]:
        """
        Validate a data file against its schema.
        
        Args:
            data_name: Name of data file (without .json extension)
            schema_name: Name of schema file (without .json extension)
            
        Returns:
            Tuple of (is_valid, error_messages)
        """
        try:
            data = self.load_data(data_name)
            schema = self.load_schema(schema_name)
            
            validate(instance=data, schema=schema)
            return True, []
            
        except ValidationError as e:
            return False, [f"Validation error: {e.message}"]
        except FileNotFoundError as e:
            return False, [f"File not found: {e}"]
        except json.JSONDecodeError as e:
            return False, [f"Invalid JSON: {e}"]
        except Exception as e:
            return False, [f"Unexpected error: {e}"]
    
    def validate_all(self) -> Dict[str, Tuple[bool, List[str]]]:
        """
        Validate all known data files against their schemas.
        
        Returns:
            Dictionary mapping data file names to (is_valid, errors) tuples
        """
        validations = {
            "ship_types": "ship_types_schema",
            "style_descriptors": "style_descriptors_schema",
            "facilities": "facilities_schema",
        }
        
        results = {}
        for data_name, schema_name in validations.items():
            results[data_name] = self.validate_file(data_name, schema_name)
        
        return results
    
    def print_validation_report(self):
        """Print a formatted validation report for all data files."""
        results = self.validate_all()
        
        print("="*60)
        print("ProgShip Data Validation Report")
        print("="*60)
        
        all_valid = True
        for data_name, (is_valid, errors) in results.items():
            status = "✓ PASS" if is_valid else "✗ FAIL"
            print(f"\n{status}: {data_name}.json")
            
            if not is_valid:
                all_valid = False
                for error in errors:
                    print(f"  - {error}")
        
        print("\n" + "="*60)
        if all_valid:
            print("✓ All validations passed")
        else:
            print("✗ Some validations failed")
        print("="*60)
        
        return all_valid


if __name__ == "__main__":
    validator = SchemaValidator()
    validator.print_validation_report()
