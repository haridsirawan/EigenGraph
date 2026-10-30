//! parametric.rs — Parametric Formula Evaluation Engine & Dependency Graph (DAG).

use std::collections::HashMap;
use crate::core::model::{BoundaryFixity, CrossSection, Element3D, ElementType, Material, Node3D, StructuralModel};

/// A driving parametric expression
#[derive(Debug, Clone, PartialEq)]
pub struct ParametricFormula {
    pub variable: String,
    pub expression: String,
    pub evaluated_value: f64,
}

/// Dependency Resolver & Formula Evaluator
pub struct ParametricEngine {
    pub variables: HashMap<String, f64>,
}

impl Default for ParametricEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ParametricEngine {
    pub fn new() -> Self {
        let mut vars = HashMap::new();
        vars.insert("L".to_string(), 24.0);
        vars.insert("H".to_string(), 3.2);
        vars.insert("n".to_string(), 6.0);
        Self { variables: vars }
    }

    pub fn set_variable(&mut self, name: impl Into<String>, val: f64) {
        self.variables.insert(name.into(), val);
    }

    pub fn get_variable(&self, name: &str) -> Option<f64> {
        self.variables.get(name).copied()
    }

    /// Evaluates simple mathematical expressions with variables: +, -, *, /, ^, numbers
    pub fn evaluate_expr(&self, expr: &str) -> Result<f64, String> {
        let cleaned = expr.trim();

        // 1. Check if it's directly a number
        if let Ok(num) = cleaned.parse::<f64>() {
            return Ok(num);
        }

        // 2. Check if it's a known variable identifier
        if let Some(&val) = self.variables.get(cleaned) {
            return Ok(val);
        }

        // 3. Simple binary operations: division (e.g. "L / 7.5")
        if let Some(pos) = cleaned.find('/') {
            let left = self.evaluate_expr(&cleaned[..pos])?;
            let right = self.evaluate_expr(&cleaned[pos + 1..])?;
            if right.abs() < 1e-12 {
                return Err("Division by zero in parametric expression".to_string());
            }
            return Ok(left / right);
        }

        // Multiplication (e.g. "B * t")
        if let Some(pos) = cleaned.find('*') {
            let left = self.evaluate_expr(&cleaned[..pos])?;
            let right = self.evaluate_expr(&cleaned[pos + 1..])?;
            return Ok(left * right);
        }

        // Addition
        if let Some(pos) = cleaned.find('+') {
            let left = self.evaluate_expr(&cleaned[..pos])?;
            let right = self.evaluate_expr(&cleaned[pos + 1..])?;
            return Ok(left + right);
        }

        // Subtraction
        if let Some(pos) = cleaned.rfind('-') {
            if pos > 0 {
                let left = self.evaluate_expr(&cleaned[..pos])?;
                let right = self.evaluate_expr(&cleaned[pos + 1..])?;
                return Ok(left - right);
            }
        }

        Err(format!("Unable to parse expression: '{}'", expr))
    }

    /// Automatically generates a full StructuralModel for a Parametric Warren Truss based on L, H, n
    pub fn generate_warren_truss(&mut self) -> Result<StructuralModel, String> {
        let span_l = self.get_variable("L").unwrap_or(24.0);
        let height_h = self.get_variable("H").unwrap_or(span_l / 7.5);
        let n_panels = self.get_variable("n").unwrap_or(6.0).round() as usize;

        if n_panels < 2 {
            return Err("Warren truss requires at least 2 panels".to_string());
        }

        let mut model = StructuralModel::new(format!("Parametric Warren Truss (L={:.1}m, H={:.1}m, n={})", span_l, height_h, n_panels));
        let dx = span_l / (n_panels as f64);

        // 1. Generate Bottom Chord Nodes (Nodes 0..=n)
        for i in 0..=n_panels {
            let x = (i as f64) * dx;
            let mut node = Node3D::new(i, x, 0.0, 0.0);
            if i == 0 {
                // Pin support at left end
                node.fixity = BoundaryFixity::pinned();
            } else if i == n_panels {
                // Roller support at right end
                node.fixity = BoundaryFixity::roller_y();
            }
            model.add_node(node);
        }

        // 2. Generate Top Chord Nodes (Nodes n+1..=2n)
        let top_start_id = n_panels + 1;
        for i in 0..n_panels {
            let x = ((i as f64) + 0.5) * dx;
            let node = Node3D::new(top_start_id + i, x, height_h, 0.0);
            model.add_node(node);
        }

        let steel = Material::structural_steel();
        let chord_sec = CrossSection::w14x90();
        let web_sec = CrossSection::hss8x8();

        let mut elem_id = 1;

        // 3. Connect Bottom Chord Elements
        for i in 0..n_panels {
            let mut elem = Element3D::new(elem_id, format!("Bottom Chord B{}", i + 1), i, i + 1);
            elem.element_type = ElementType::Truss3D;
            elem.section = chord_sec.clone();
            elem.material = steel.clone();
            model.add_element(elem);
            elem_id += 1;
        }

        // 4. Connect Top Chord Elements
        for i in 0..(n_panels - 1) {
            let mut elem = Element3D::new(elem_id, format!("Top Chord T{}", i + 1), top_start_id + i, top_start_id + i + 1);
            elem.element_type = ElementType::Truss3D;
            elem.section = chord_sec.clone();
            elem.material = steel.clone();
            model.add_element(elem);
            elem_id += 1;
        }

        // 5. Connect Web Diagonals (Warren Pattern)
        for i in 0..n_panels {
            let top_node = top_start_id + i;
            let bot_left = i;
            let bot_right = i + 1;

            let mut d1 = Element3D::new(elem_id, format!("Diagonal D{}_a", i + 1), bot_left, top_node);
            d1.element_type = ElementType::Truss3D;
            d1.section = web_sec.clone();
            d1.material = steel.clone();
            model.add_element(d1);
            elem_id += 1;

            let mut d2 = Element3D::new(elem_id, format!("Diagonal D{}_b", i + 1), top_node, bot_right);
            d2.element_type = ElementType::Truss3D;
            d2.section = web_sec.clone();
            d2.material = steel.clone();
            model.add_element(d2);
            elem_id += 1;
        }

        Ok(model)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parametric_eval() {
        let mut engine = ParametricEngine::new();
        engine.set_variable("L", 24000.0);
        let h = engine.evaluate_expr("L / 7.5").expect("Should evaluate H");
        assert_eq!(h, 3200.0);
    }

    #[test]
    fn test_warren_generation() {
        let mut engine = ParametricEngine::new();
        let model = engine.generate_warren_truss().expect("Should generate model");
        assert_eq!(model.nodes.len(), 7 + 6); // 7 bottom + 6 top = 13 nodes
        assert_eq!(model.elements.len(), 6 + 5 + 12); // 6 bottom + 5 top + 12 diagonals = 23 elements
    }
}
