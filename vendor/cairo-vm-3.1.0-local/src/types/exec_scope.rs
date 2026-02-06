use crate::stdlib::{any::Any, cell::RefCell, collections::HashMap, prelude::*, rc::Rc};
use crate::{
    any_box,
    hint_processor::builtin_hint_processor::dict_manager::DictManager,
    vm::errors::{exec_scope_errors::ExecScopeError, hint_errors::HintError},
};

#[derive(Debug)]
pub struct ExecutionScopes {
    pub data: Vec<HashMap<String, Box<dyn Any>>>,
}

impl ExecutionScopes {
    pub fn new() -> ExecutionScopes {
        ExecutionScopes {
            data: vec![HashMap::new()],
        }
    }

    pub fn enter_scope(&mut self, new_scope_locals: HashMap<String, Box<dyn Any>>) {
        self.data.push(new_scope_locals);
    }

    pub fn exit_scope(&mut self) -> Result<(), ExecScopeError> {
        if self.data.len() == 1 {
            return Err(ExecScopeError::ExitMainScopeError);
        }
        self.data.pop();

        Ok(())
    }

    ///Returns a mutable reference to the dictionary containing the variables present in the current scope
    pub fn get_local_variables_mut(
        &mut self,
    ) -> Result<&mut HashMap<String, Box<dyn Any>>, HintError> {
        self.data
            .last_mut()
            .ok_or(HintError::FromScopeError(ExecScopeError::NoScopeError))
    }

    ///Returns a dictionary containing the variables present in the current scope
    pub fn get_local_variables(&self) -> Result<&HashMap<String, Box<dyn Any>>, HintError> {
        self.data
            .last()
            .ok_or(HintError::FromScopeError(ExecScopeError::NoScopeError))
    }

    ///Removes a variable from the current scope given its name
    pub fn delete_variable(&mut self, var_name: &str) {
        if let Ok(local_variables) = self.get_local_variables_mut() {
            local_variables.remove(var_name);
        }
    }

    ///Creates or updates an existing variable given its name and boxed value
    pub fn assign_or_update_variable(&mut self, var_name: &str, var_value: Box<dyn Any>) {
        if let Ok(local_variables) = self.get_local_variables_mut() {
            local_variables.insert(var_name.to_string(), var_value);
        }
    }

    ///Returns the value in the current execution scope that matches the name and is of the given generic type
    pub fn get<T: Any + Clone>(&self, name: &str) -> Result<T, HintError> {
        let mut val: Option<T> = None;
        if let Some(variable) = self.get_local_variables()?.get(name) {
            if let Some(int) = variable.downcast_ref::<T>() {
                val = Some(int.clone());
            }
        }
        val.ok_or_else(|| HintError::VariableNotInScopeError(name.to_string().into_boxed_str()))
    }

    ///Returns a reference to the value in the current execution scope that matches the name and is of the given generic type
    pub fn get_ref<T: Any>(&self, name: &str) -> Result<&T, HintError> {
        let mut val: Option<&T> = None;
        if let Some(variable) = self.get_local_variables()?.get(name) {
            if let Some(int) = variable.downcast_ref::<T>() {
                val = Some(int);
            }
        }
        val.ok_or_else(|| HintError::VariableNotInScopeError(name.to_string().into_boxed_str()))
    }

    ///Returns a mutable reference to the value in the current execution scope that matches the name and is of the given generic type
    pub fn get_mut_ref<T: Any>(&mut self, name: &str) -> Result<&mut T, HintError> {
        let mut val: Option<&mut T> = None;
        if let Some(variable) = self.get_local_variables_mut()?.get_mut(name) {
            if let Some(int) = variable.downcast_mut::<T>() {
                val = Some(int);
            }
        }
        val.ok_or_else(|| HintError::VariableNotInScopeError(name.to_string().into_boxed_str()))
    }

    ///Returns the value in the current execution scope that matches the name
    pub fn get_any_boxed_ref(&self, name: &str) -> Result<&Box<dyn Any>, HintError> {
        if let Some(variable) = self.get_local_variables()?.get(name) {
            return Ok(variable);
        }
        Err(HintError::VariableNotInScopeError(
            name.to_string().into_boxed_str(),
        ))
    }

    ///Returns the value in the current execution scope that matches the name
    pub fn get_any_boxed_mut(&mut self, name: &str) -> Result<&mut Box<dyn Any>, HintError> {
        if let Some(variable) = self.get_local_variables_mut()?.get_mut(name) {
            return Ok(variable);
        }
        Err(HintError::VariableNotInScopeError(
            name.to_string().into_boxed_str(),
        ))
    }

    ///Returns the value in the current execution scope that matches the name and is of type List
    pub fn get_list<T: Any + Clone>(&self, name: &str) -> Result<Vec<T>, HintError> {
        let mut val: Option<Vec<T>> = None;
        if let Some(variable) = self.get_local_variables()?.get(name) {
            if let Some(list) = variable.downcast_ref::<Vec<T>>() {
                val = Some(list.clone());
            }
        }
        val.ok_or_else(|| HintError::VariableNotInScopeError(name.to_string().into_boxed_str()))
    }

    ///Returns a reference to the value in the current execution scope that matches the name and is of type List
    pub fn get_list_ref<T: Any>(&self, name: &str) -> Result<&Vec<T>, HintError> {
        let mut val: Option<&Vec<T>> = None;
        if let Some(variable) = self.get_local_variables()?.get(name) {
            if let Some(list) = variable.downcast_ref::<Vec<T>>() {
                val = Some(list);
            }
        }
        val.ok_or_else(|| HintError::VariableNotInScopeError(name.to_string().into_boxed_str()))
    }

    ///Returns a mutable reference to the value in the current execution scope that matches the name and is of type List
    pub fn get_mut_list_ref<T: Any>(&mut self, name: &str) -> Result<&mut Vec<T>, HintError> {
        let mut val: Option<&mut Vec<T>> = None;
        if let Some(variable) = self.get_local_variables_mut()?.get_mut(name) {
            if let Some(list) = variable.downcast_mut::<Vec<T>>() {
                val = Some(list);
            }
        }
        val.ok_or_else(|| HintError::VariableNotInScopeError(name.to_string().into_boxed_str()))
    }

    ///Returns the value in the dict manager
    pub fn get_dict_manager(&self) -> Result<Rc<RefCell<DictManager>>, HintError> {
        let mut val: Option<Rc<RefCell<DictManager>>> = None;
        if let Some(variable) = self.get_local_variables()?.get("dict_manager") {
            if let Some(dict_manager) = variable.downcast_ref::<Rc<RefCell<DictManager>>>() {
                val = Some(dict_manager.clone());
            }
        }
        val.ok_or_else(|| {
            HintError::VariableNotInScopeError("dict_manager".to_string().into_boxed_str())
        })
    }

    ///Returns a mutable reference to the value in the current execution scope that matches the name and is of the given type
    pub fn get_mut_dict_ref<K: Any, V: Any>(
        &mut self,
        name: &str,
    ) -> Result<&mut HashMap<K, V>, HintError> {
        let mut val: Option<&mut HashMap<K, V>> = None;
        if let Some(variable) = self.get_local_variables_mut()?.get_mut(name) {
            if let Some(dict) = variable.downcast_mut::<HashMap<K, V>>() {
                val = Some(dict);
            }
        }
        val.ok_or_else(|| HintError::VariableNotInScopeError(name.to_string().into_boxed_str()))
    }

    ///Inserts the boxed value into the current scope
    pub fn insert_box(&mut self, name: &str, value: Box<dyn Any>) {
        self.assign_or_update_variable(name, value);
    }

    ///Inserts the value into the current scope
    pub fn insert_value<T: 'static>(&mut self, name: &str, value: T) {
        self.assign_or_update_variable(name, any_box!(value));
    }
}

impl Default for ExecutionScopes {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Felt252;
    use assert_matches::assert_matches;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn initialize_execution_scopes() {
        let scopes = ExecutionScopes::new();
        assert_eq!(scopes.data.len(), 1);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_local_variables_test() {
        let var_name = String::from("a");
        let var_value: Box<dyn Any> = Box::new(Felt252::from(2));

        let scope = HashMap::from([(var_name, var_value)]);

        let scopes = ExecutionScopes { data: vec![scope] };
        assert_eq!(scopes.get_local_variables().unwrap().len(), 1);
        assert_eq!(
            scopes
                .get_local_variables()
                .unwrap()
                .get("a")
                .unwrap()
                .downcast_ref::<Felt252>(),
            Some(&Felt252::from(2))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn enter_new_scope_test() {
        let var_name = String::from("a");
        let var_value: Box<dyn Any> = Box::new(Felt252::from(2_i32));

        let new_scope = HashMap::from([(var_name, var_value)]);

        let mut scopes = ExecutionScopes {
            data: vec![HashMap::from([(
                String::from("b"),
                (Box::new(Felt252::ONE) as Box<dyn Any>),
            )])],
        };

        assert_eq!(scopes.get_local_variables().unwrap().len(), 1);
        assert_eq!(
            scopes
                .get_local_variables()
                .unwrap()
                .get("b")
                .unwrap()
                .downcast_ref::<Felt252>(),
            Some(&Felt252::ONE)
        );

        scopes.enter_scope(new_scope);

        // check that variable `b` can't be accessed now
        assert!(scopes.get_local_variables().unwrap().get("b").is_none());

        assert_eq!(scopes.get_local_variables().unwrap().len(), 1);
        assert_eq!(
            scopes
                .get_local_variables()
                .unwrap()
                .get("a")
                .unwrap()
                .downcast_ref::<Felt252>(),
            Some(&Felt252::from(2))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn exit_scope_test() {
        let var_name = String::from("a");
        let var_value: Box<dyn Any> = Box::new(Felt252::from(2));

        let new_scope = HashMap::from([(var_name, var_value)]);

        // this initializes an empty main scope
        let mut scopes = ExecutionScopes::new();

        // enter one extra scope
        scopes.enter_scope(new_scope);

        assert_eq!(scopes.get_local_variables().unwrap().len(), 1);
        assert_eq!(
            scopes
                .get_local_variables()
                .unwrap()
                .get("a")
                .unwrap()
                .downcast_ref::<Felt252>(),
            Some(&Felt252::from(2))
        );

        // exit the current scope
        let exit_scope_result = scopes.exit_scope();

        assert!(exit_scope_result.is_ok());

        // assert that variable `a` is no longer available
        assert!(scopes.get_local_variables().unwrap().get("a").is_none());

        // assert that we recovered the older scope
        assert!(scopes.get_local_variables().unwrap().is_empty());
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn assign_local_variable_test() {
        let var_value: Box<dyn Any> = Box::new(Felt252::from(2));

        let mut scopes = ExecutionScopes::new();

        scopes.assign_or_update_variable("a", var_value);

        assert_eq!(scopes.get_local_variables().unwrap().len(), 1);
        assert_eq!(
            scopes
                .get_local_variables()
                .unwrap()
                .get("a")
                .unwrap()
                .downcast_ref::<Felt252>(),
            Some(&Felt252::from(2))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn re_assign_local_variable_test() {
        let var_name = String::from("a");
        let var_value: Box<dyn Any> = Box::new(Felt252::from(2));

        let scope = HashMap::from([(var_name, var_value)]);

        let mut scopes = ExecutionScopes { data: vec![scope] };

        let var_value_new: Box<dyn Any> = Box::new(Felt252::from(3));

        scopes.assign_or_update_variable("a", var_value_new);

        assert_eq!(scopes.get_local_variables().unwrap().len(), 1);
        assert_eq!(
            scopes
                .get_local_variables()
                .unwrap()
                .get("a")
                .unwrap()
                .downcast_ref::<Felt252>(),
            Some(&Felt252::from(3))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn delete_local_variable_test() {
        let var_name = String::from("a");
        let var_value: Box<dyn Any> = Box::new(Felt252::from(2));

        let scope = HashMap::from([(var_name, var_value)]);

        let mut scopes = ExecutionScopes { data: vec![scope] };

        assert!(scopes
            .get_local_variables()
            .unwrap()
            .contains_key(&String::from("a")));

        scopes.delete_variable("a");

        assert!(!scopes
            .get_local_variables()
            .unwrap()
            .contains_key(&String::from("a")));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn exit_main_scope_gives_error_test() {
        let mut scopes = ExecutionScopes::new();

        assert!(scopes.exit_scope().is_err());
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_listu64_test() {
        let list_u64: Box<dyn Any> = Box::new(vec![20_u64, 18_u64]);

        let mut scopes = ExecutionScopes::default();

        scopes.insert_box("list_u64", list_u64);

        assert_matches!(
            scopes.get_list::<u64>("list_u64"),
            Ok(x) if x == vec![20_u64, 18_u64]
        );

        assert_matches!(
            scopes.get_list::<u64>("no_variable"),
            Err(HintError::VariableNotInScopeError(
                x
            )) if *x == *"no_variable".to_string()
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_u64_test() {
        let u64: Box<dyn Any> = Box::new(9_u64);

        let mut scopes = ExecutionScopes::new();

        scopes.assign_or_update_variable("u64", u64);

        assert_matches!(scopes.get_ref::<u64>("u64"), Ok(&9_u64));
        assert_matches!(scopes.get_mut_ref::<u64>("u64"), Ok(&mut 9_u64));

        assert_matches!(
            scopes.get_mut_ref::<u64>("no_variable"),
            Err(HintError::VariableNotInScopeError(
                x
            )) if *x == *"no_variable".to_string()
        );
        assert_matches!(
            scopes.get_ref::<u64>("no_variable"),
            Err(HintError::VariableNotInScopeError(
                x
            )) if *x == *"no_variable".to_string()
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_mut_int_ref_test() {
        let bigint: Box<dyn Any> = Box::new(Felt252::from(12));

        let mut scopes = ExecutionScopes::new();
        scopes.assign_or_update_variable("bigint", bigint);

        assert_matches!(
            scopes.get_mut_ref::<Felt252>("bigint"),
            Ok(x) if x == &mut Felt252::from(12)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_any_boxed_test() {
        let list_u64: Box<dyn Any> = Box::new(vec![20_u64, 18_u64]);

        let mut scopes = ExecutionScopes::default();

        scopes.assign_or_update_variable("list_u64", list_u64);

        assert_eq!(
            scopes
                .get_any_boxed_ref("list_u64")
                .unwrap()
                .downcast_ref::<Vec<u64>>(),
            Some(&vec![20_u64, 18_u64])
        );

        assert_eq!(
            scopes
                .get_any_boxed_mut("list_u64")
                .unwrap()
                .downcast_mut::<Vec<u64>>(),
            Some(&mut vec![20_u64, 18_u64])
        );

        assert!(scopes.get_any_boxed_mut("no_variable").is_err());
        assert!(scopes.get_any_boxed_ref("no_variable").is_err());
    }
}
