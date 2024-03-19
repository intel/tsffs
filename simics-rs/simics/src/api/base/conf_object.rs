// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::not_unsafe_ptr_arg_deref, clippy::too_many_arguments)]

//! Bindings for configuration objects

use crate::{
    last_error, simics_exception,
    sys::{
        attr_attr_t, attr_value_t, class_data_t, class_info_t, class_kind_t, conf_class_t,
        conf_object_t, get_attr_t, get_class_attr_t, object_iter_t, set_attr_t, set_class_attr_t,
        set_error_t, SIM_attribute_error, SIM_copy_class, SIM_create_class, SIM_extend_class,
        SIM_extension_data, SIM_get_class_data, SIM_get_class_interface, SIM_get_class_name,
        SIM_get_interface, SIM_marked_for_deletion, SIM_object_data, SIM_object_descendant,
        SIM_object_id, SIM_object_is_configured, SIM_object_iterator, SIM_object_iterator_next,
        SIM_object_name, SIM_object_parent, SIM_register_attribute_with_user_data,
        SIM_register_class_alias, SIM_register_class_attribute_with_user_data,
        SIM_register_interface, SIM_register_typed_attribute, SIM_register_typed_class_attribute,
        SIM_require_object, SIM_set_class_data, SIM_set_object_configured,
        SIM_shallow_object_iterator,
    },
    AttrValue, Error, Interface, Result,
};
use raw_cstr::{raw_cstr, AsRawCstr};
use std::{
    ffi::{c_void, CStr},
    fmt::Display,
    ops::Range,
    ptr::null_mut,
};

/// Alias for `conf_object_t`
pub type ConfObject = conf_object_t;
/// Alias for `conf_class_t`
pub type ConfClass = conf_class_t;
/// Alias for `class_data_t`
pub type ClassData = class_data_t;
/// Alias for `class_info_t`
pub type ClassInfo = class_info_t;
/// Alias for `class_kind_t`
pub type ClassKind = class_kind_t;
/// Alias for `attr_attr_t`
pub type AttrAttr = attr_attr_t;
/// Alias for `object_iter_t`
pub type ObjectIter = object_iter_t;
/// Alias for `get_attr_t`
pub type GetAttr = get_attr_t;
/// Alias for `set_attr_t`
pub type SetAttr = set_attr_t;
/// Alias for `get_class_attr_t`
pub type GetClassAttr = get_class_attr_t;
/// Alias for `set_class_attr_t`
pub type SetClassAttr = set_class_attr_t;
/// Alias for `set_error_t`
pub type SetErr = set_error_t;

/// A type in a [`TypeStringType::List`]. See [`TypeStringType`] for a description of these
/// variants.
pub enum TypeStringListType {
    /// A single type
    Type(Box<TypeStringType>),
    /// A sequence of types with a range of possible lengths
    Range(Range<usize>, Box<TypeStringType>),
    /// A sequence of types with an exact length
    Exact(usize, Box<TypeStringType>),
    /// A sequence of zero or more occurrences of a type
    ZeroOrMore(Box<TypeStringType>),
    /// A sequence of one or more occurrences of a type
    OneOrMore(Box<TypeStringType>),
}

impl Display for TypeStringListType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeStringListType::Type(t) => write!(f, "{}", t),
            TypeStringListType::Range(r, t) => {
                write!(f, "{}{{{}:{}}}", t, r.start, r.end)
            }
            TypeStringListType::Exact(c, t) => write!(f, "{}{{{}}}", t, c),
            TypeStringListType::ZeroOrMore(t) => write!(f, "{}*", t),
            TypeStringListType::OneOrMore(t) => write!(f, "{}+", t),
        }
    }
}

/// A type in a python-like type string
///
/// The enumeration represents a type string
///
/// Most types are represented by a single letter:
///
/// | Letter | Type           |
/// | ------ | -------------- |
/// | i      | integer        |
/// | f      | floating-point |
/// | s      | string         |
/// | b      | boolean        |
/// | o      | object         |
/// | d      | data           |
/// | n      | nil            |
/// | D      | dictionary     |
/// | a      | any type       |
///
/// The | (vertical bar) operator specifies the union of two types; eg, s|o is the type
/// of a string or an object.  Lists are defined inside square brackets: []. There are
/// two kinds of list declarations:
///
/// A heterogeneous list of fixed length is defined by the types of its elements. For
/// example, [ios] specifies a 3-element list consisting of an integer, an object and a
/// string, in that order.
///
/// A homogeneous list of varying length is defined by a single type followed by a
/// length modifier:
///
/// | Modifier | Meaning                             |
/// | -------- | ----------------------------------- |
/// | {N:M}    | between N and M elements, inclusive |
/// | {N}      | exactly N elements                  |
/// | *        | zero or more elements               |
/// | +        | one or more elements                |
///
/// For example, [i{3,5}] specifies a list of 3, 4 or 5 integers.
///
/// Inside heterogeneous lists, | (union) has higher precedence than juxtaposition; ie,
/// [i|so|n] defines a list of two elements, the first being an integer or a string and
/// the second an object or NIL.
pub enum TypeStringType {
    /// An integer type
    Integer,
    /// A floating point type
    Float,
    /// A string type
    String,
    /// A boolean type
    Boolean,
    /// An object type, i.e. a pointer to a confobject
    Object,
    /// A data type, i.e. a void pointer
    Data,
    /// nil (i.e. None) type
    Nil,
    /// A dictionary or mapping type
    Dictionary,
    /// Any type
    Any,
    /// A list of types
    List(Vec<TypeStringListType>),
    /// An alternation, either the left or right type is permitted
    Or(Box<TypeStringType>, Box<TypeStringType>),
}

impl Display for TypeStringType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeStringType::Integer => write!(f, "i"),
            TypeStringType::Float => write!(f, "f"),
            TypeStringType::String => write!(f, "s"),
            TypeStringType::Boolean => write!(f, "b"),
            TypeStringType::Object => write!(f, "o"),
            TypeStringType::Data => write!(f, "d"),
            TypeStringType::Nil => write!(f, "n"),
            TypeStringType::Dictionary => write!(f, "D"),
            TypeStringType::Any => write!(f, "a"),
            TypeStringType::List(l) => write!(
                f,
                "[{}]",
                l.iter().map(|li| li.to_string()).collect::<String>()
            ),
            TypeStringType::Or(l, r) => write!(f, "{}|{}", l, r),
        }
    }
}

// NOTE: There is an old class creation method, but it is *actually* deprecated, so we do not
// include it with a #[deprecated] warning.

#[simics_exception]
/// Register an alias alias for the existing class class_name. Using aliases allows the
/// read-configuration command to read configuration files that define objects of type
/// alias, while the write-configuration command always uses class_name.
///
/// Aliases are used to support compatibility with old class names if a class is
/// renamed. They can also be used to allow different modules, which define different
/// specific implementations of the same generic base class, to read the same
/// configuration files.
///
/// # Arguments
///
/// * `alias` - The name to register as an alias for the class that is already registered for `name`
/// * `name` The name of the class to register an alias for
///
/// # Return value
///
/// Ok if the alias was registered successfully, or an error otherwise.
///
/// # Context
///
/// Global Context
pub fn register_class_alias<S>(alias: S, name: S) -> Result<()>
where
    S: AsRef<str>,
{
    unsafe { SIM_register_class_alias(raw_cstr(alias)?, raw_cstr(name)?) };
    Ok(())
}

#[simics_exception]
/// This function creates a new class that can be instantiated by calling the
/// SIM_create_object function. It is a replacement for SIM_register_class and should be
/// used in all new code.
///
/// The name can contain upper and lower case ASCII letters, hyphens, underscores, and
/// digits. It must not begin with a digit or a hyphen and must not end with a hyphen.
///
/// # Arguments
///
/// * `name` - The name to register the class for
/// * `class_info` - The description of the class
///
/// # Return Value
///
/// A pointer to the successfully registered class object, or an error if registration
/// is not successful
///
/// # Context
///
/// Global Context
pub fn create_class<S>(name: S, class_info: ClassInfo) -> Result<*mut ConfClass>
where
    S: AsRef<str>,
{
    let name_raw = raw_cstr(name.as_ref())?;

    // The reference can be dropped after the `SIM_create_class` function returns,
    // so this is safe to call this way
    let cls = unsafe { SIM_create_class(name_raw, &class_info as *const ClassInfo) };

    if cls.is_null() {
        Err(Error::CreateClass {
            name: name.as_ref().to_string(),
            message: last_error(),
        })
    } else {
        Ok(cls)
    }
}

#[simics_exception]
/// The function extends the class cls with attributes, interfaces, port objects and
/// port interfaces defined by the extension class ext.
///
/// The extension class must be of the type Sim_Class_Kind_Extension and must not define
/// any attributes or interfaces which have already been defined by the class being
/// augmented.
///
/// Besides normal object initialization, the init_object method for the extension
/// class, will be called when cls is instantiated. The pointer returned by init_object
/// can be retrieved using SIM_extension_data. The init_object method may return NULL if
/// no private data pointer is needed; this does not signify an error condition for
/// extension classes.
///
/// The finalize_instance method defined by the extension class will be called before
/// the finalize_instance method is called for the class being extended.
///
/// The SIM_extension_class function is intended to be used to extend a class with
/// generic functionality, common to multiple classes.
///
/// # Arguments
///
/// * `cls` - The class to extend
/// * `ext` - The extension class to extend the class with
///
/// # Context
///
/// Global Context
pub fn extend_class(cls: *mut ConfClass, ext: *mut ConfClass) {
    unsafe { SIM_extend_class(cls, ext) };
}

#[simics_exception]
/// This function creates a copy of the class src_class named name.  Additional
/// attributes and interfaces can be registered on the newly created class.
///
/// The new class is described by desc
///
/// # Arguments
///
/// * `name` - The name of the new class
/// * `src_cls` - The class to make a copy of
/// * `desc` - The description string of the new class
///
/// # Return Value
///
/// The new copied class
///
/// # Context
///
/// Global Context
pub fn copy_class<S>(name: S, src_cls: *mut ConfClass, desc: S) -> Result<*mut ConfClass>
where
    S: AsRef<str>,
{
    Ok(unsafe { SIM_copy_class(raw_cstr(name)?, src_cls, raw_cstr(desc)?) })
}

#[simics_exception]
/// Get the name of a class. The name is copied, which differs from the
/// C API.
///
/// # Arguments
///
/// * `cls` - The class to get the name of
///
/// # Return Value
///
/// The name of the class
///
/// # Context
///
/// Cell Context
pub fn get_class_name(cls: *mut ConfClass) -> Result<String> {
    Ok(unsafe { CStr::from_ptr(SIM_get_class_name(cls)) }
        .to_str()
        .map(|s| s.to_string())?)
}

#[simics_exception]
/// Set extra data for the specified class. This is particularly useful if the same class
/// methods are used for multiple distinct classes, for instance for generated classes.
/// The class data can be fetched at any time during the object initialisation, using
/// [`get_class_data`].
///
/// # Arguments
///
/// * `cls` - The class to set extra data for
/// * `data` - Extra data to store
///
/// # Context
///
/// Global Context
pub fn set_class_data<T>(cls: *mut ConfClass, data: T) {
    unsafe { SIM_set_class_data(cls, Box::into_raw(Box::new(data)) as *mut c_void) }
}

#[simics_exception]
/// Obtain the class data that was set using [`set_class_data`]. This can be called at
/// any time during the object initialisation process.
///
/// # Arguments
///
/// * `cls` - The class to retrieve data for.
///
/// # Return Value
///
/// The class data. Ownership of the data is not transferred to the caller. If `T`
/// implements `Clone`, clone the data to obtain an owned object.
///
/// # Context
///
/// Cell Context
pub fn get_class_data<'a, T>(cls: *mut ConfClass) -> &'a mut T {
    unsafe { Box::leak(Box::from_raw(SIM_get_class_data(cls) as *mut T)) }
}

#[simics_exception]
/// If obj has not yet been set as configured, then that object's finalize method
/// (post_init in DML) is run; otherwise, nothing happens. After completion of that
/// method, obj will be set as configured.
///
/// Each object will have its finalize method called automatically, usually in
/// hierarchical order, during object creation. Since it is only permitted to call
/// methods on objects that have been configured, [`require_object`] is a way to allow
/// such calls during finalisation by ensuring that those objects are correctly set up.
/// A better way to call methods on other objects during finalization is to defer such
/// calls to the objects_finalized method.
///
/// [`require_object`] may only be called from the finalize method of another object.
///
/// Finalisation cycles can occur if two or more objects call [`require_object`] on each
/// other. Such cycles are treated as errors. To avoid them, call
/// [`set_object_configured`] as soon as the object has reached a consistent state.
///
/// # Arguments
///
/// * `obj` - The object to require finalization for
///
/// # Context
///
/// Global Context
pub fn require_object(obj: *mut ConfObject) {
    unsafe { SIM_require_object(obj) };
}

#[simics_exception]
/// Returns the name of an object. This name identifies the object uniquely, but may
/// change if the object is moved to another hierarchical location.
///
/// The return value is a string, owned by obj, that should not be modified or freed by
/// the caller.
///
/// # Arguments
///
/// * `obj` - The object to get the name for
///
/// # Return Value
///
/// The unique name of the object
///
/// # Context
///
/// All Contexts
pub fn object_name(obj: *mut ConfObject) -> Result<String> {
    Ok(unsafe { CStr::from_ptr(SIM_object_name(obj)) }
        .to_str()
        .map(|s| s.to_string())?)
}

#[simics_exception]
/// Returns the unique identifier for an object. The identifier is a string that is
/// guaranteed to be unique and will never change, even if the object moves to another
/// hierarchical location.
///
/// The return value is a static string that should not be modified or freed by the
/// caller.
///
/// # Arguments
///
/// * `obj` - The object to get the id for
///
/// # Return Value
///
/// The unique id of the object
///
/// # Context
///
/// All Contexts
pub fn object_id(obj: *mut ConfObject) -> Result<String> {
    Ok(unsafe { CStr::from_ptr(SIM_object_id(obj)) }
        .to_str()
        .map(|s| s.to_string())?)
}

#[simics_exception]
/// [`object_is_configured`] indicates whether obj is configured.
///
/// An object is configured once its finalize_instance method (post_init in DML) has
/// completed, or [`set_object_configured`] has been called for it. Being configured
/// indicates that the object is in a consistent state and is ready to be used by other
/// objects.
///
/// # Arguments
///
/// * `obj` - The object to retrieve the configured status for
///
/// # Return Value
///
/// Whether the object is configured
///
/// # Context
///
/// All Contexts
pub fn object_is_configured(obj: *mut ConfObject) -> bool {
    unsafe { SIM_object_is_configured(obj) }
}

#[simics_exception]
/// [`set_object_configured`] sets the object as configured.
///
/// [`set_object_configured`] is used to avoid circular dependencies between objects. It
/// may only be called from the object's own finalize_instance method, when the object
/// is known to be in a consistent state.
///
/// # Arguments
///
/// * `obj` - The object to set as configured
///
/// # Context
///
/// Global Context
pub fn set_object_configured(obj: *mut ConfObject) {
    unsafe { SIM_set_object_configured(obj) }
}

#[simics_exception]
/// Returns the private data pointer of an object. This pointer is available to the
/// class for storing instance-specific state.  It is initialised to the return value of
/// the init (from class_info_t) method that is called during object creation. For
/// classes created using the legacy [`register_class`], the same functionality is
/// provided by the init_object method .
///
/// For classes implemented in Python, the data (which is then a Python value) can also
/// be accessed as obj.object_data.
///
/// For classes written in C, the preferred way to store instance-specific state is by
/// co-allocation with the object's conf_object_t structure instead of using
/// [`object_data`]. Such classes should define the alloc method in the class_info_t
/// passed to [`create_class`] for allocating its instance data. For classes using the
/// legacy [`register_class`] class registration function, they should define the
/// alloc_object method in the class_data_t data structure.
///
/// # Arguments
///
/// * `obj` - The object to retrieve extra data for
///
/// # Return value
///
/// A reference to the object's inner data
///
/// # Context
///
/// All Contexts
pub fn object_data<'a, T>(obj: *mut ConfObject) -> &'a mut T {
    unsafe { Box::leak(Box::from_raw(SIM_object_data(obj) as *mut T)) }
}

#[simics_exception]
/// Returns the private data pointer of an object associated with the extension class
/// ext_cls. The returned pointer is the value returned by the init_object method called
/// for the extension class ext_cls.
///
/// The object obj must be an instance of a class which has been extended with the
/// extension class ext_cls using the [`extend_class`] function.
///
/// # Arguments
///
/// * `obj` - An instance of a class which has been extended with `cls`
/// * `cls` - The class that extends the class `obj` is an instance of
///
/// # Return value
///
/// A reference to the object's inner data
///
/// # Context
///
/// Cell Context
pub fn extension_data<'a, T>(obj: *mut ConfObject, cls: *mut ConfClass) -> &'a mut T {
    unsafe { Box::leak(Box::from_raw(SIM_extension_data(obj, cls) as *mut T)) }
}

#[simics_exception]
/// Retrieve the parent object if there is one, or None otherwise.
///
/// # Arguments
///
/// * `obj` - The object to get a parent object for
///
/// # Return Value
///
/// A pointer to the parent object if there is one, or None otherwise
///
/// # Context
///
/// Unknown
pub fn object_parent(obj: *mut ConfObject) -> Option<*mut ConfObject> {
    let ptr = unsafe { SIM_object_parent(obj) };

    if ptr.is_null() {
        None
    } else {
        Some(ptr)
    }
}

#[simics_exception]
/// Retrieve an object's descendant with a name, if one exists.
///
/// # Arguments
///
/// * `obj` - The object to get descendants for
/// * `relname` - The name of the related descendant
///
/// # Return Value
///
/// The descendant of the object with a name, if one exists
///
/// # Context
///
/// Unknown
pub fn object_descendant<S>(obj: *mut ConfObject, relname: S) -> Result<Option<*mut ConfObject>>
where
    S: AsRef<str>,
{
    let ptr = unsafe { SIM_object_descendant(obj, raw_cstr(relname)?) };
    if ptr.is_null() {
        Ok(None)
    } else {
        Ok(Some(ptr))
    }
}

#[simics_exception]
/// Obtain an iterator over the child objects at all depths of a given object
///
/// # Argument
///
/// * `obj` - The object to get an iterator for
///
/// # Return Value
///
/// The iterator over the object's children
///
/// # Context
///
/// Unknown
pub fn object_iterator(obj: *mut ConfObject) -> ObjectIter {
    unsafe { SIM_object_iterator(obj) }
}

#[simics_exception]
/// Obtain an iterator over the child objects at depth 1 of a given object
///
/// # Arguments
///
/// * `obj` - The object to get an iterator for
///
/// # Return Value
///
/// An iterator over the object's children, non-recursively
///
/// # Context
///
/// Unknown
pub fn shallow_object_iterator(obj: *mut ConfObject) -> ObjectIter {
    unsafe { SIM_shallow_object_iterator(obj) }
}

#[simics_exception]
/// Consume and return the next item of an object iterator, if one exists
///
/// # Arguments
///
/// * `iter` - The iterator obtained from [`object_iterator`] or
/// [`shallow_object_iterator`]
///
/// # Return Value
///
/// The next element in the iteration, or `None` if the iterator has been exhausted
///
/// # Context
///
/// Unknown
pub fn object_iterator_next(iter: *mut ObjectIter) -> Option<*mut ConfObject> {
    let obj = unsafe { SIM_object_iterator_next(iter) };

    if obj.is_null() {
        None
    } else {
        Some(obj)
    }
}

extern "C" fn get_typed_attr_handler<F>(
    cb: *mut c_void,
    obj: *mut ConfObject,
    idx: *mut attr_value_t,
) -> attr_value_t
where
    F: FnMut(*mut ConfObject, AttrValue) -> Result<AttrValue> + 'static,
{
    let closure = Box::leak(unsafe { Box::from_raw(cb as *mut Box<F>) });
    let idx = unsafe { AttrValue::from_raw(idx) };

    closure(obj, idx)
        .expect("Error calling get_typed_attr_handler callback")
        .into_raw()
}

extern "C" fn set_typed_attr_handler<F>(
    cb: *mut c_void,
    obj: *mut ConfObject,
    val: *mut attr_value_t,
    idx: *mut attr_value_t,
) -> SetErr
where
    F: FnMut(*mut ConfObject, AttrValue, AttrValue) -> Result<SetErr> + 'static,
{
    let closure = Box::leak(unsafe { Box::from_raw(cb as *mut Box<F>) });
    let val = unsafe { AttrValue::from_raw(val) };
    let idx = unsafe { AttrValue::from_raw(idx) };

    closure(obj, val, idx).expect("Error calling set_typed_attr_handler callback")
}

extern "C" fn get_typed_class_attr_handler<F>(
    cb: *mut c_void,
    cls: *mut ConfClass,
    idx: *mut attr_value_t,
) -> attr_value_t
where
    F: FnMut(*mut ConfClass, AttrValue) -> Result<AttrValue> + 'static,
{
    let closure = Box::leak(unsafe { Box::from_raw(cb as *mut Box<F>) });
    let idx = unsafe { AttrValue::from_raw(idx) };

    closure(cls, idx)
        .expect("Error calling get_typed_class_attr_handler callback")
        .into_raw()
}

extern "C" fn set_typed_class_attr_handler<F>(
    cb: *mut c_void,
    cls: *mut ConfClass,
    val: *mut attr_value_t,
    idx: *mut attr_value_t,
) -> SetErr
where
    F: FnMut(*mut ConfClass, AttrValue, AttrValue) -> Result<SetErr> + 'static,
{
    let closure = Box::leak(unsafe { Box::from_raw(cb as *mut Box<F>) });
    let val = unsafe { AttrValue::from_raw(val) };
    let idx = unsafe { AttrValue::from_raw(idx) };

    closure(cls, val, idx).expect("Error calling set_typed_class_attr_handler callback")
}

extern "C" fn get_attr_handler<F>(obj: *mut ConfObject, cb: *mut c_void) -> attr_value_t
where
    F: FnMut(*mut ConfObject) -> Result<AttrValue> + 'static,
{
    let closure = Box::leak(unsafe { Box::from_raw(cb as *mut Box<F>) });

    closure(obj)
        .expect("Error calling get_attr_handler callback")
        .into_raw()
}

extern "C" fn set_attr_handler<F>(
    obj: *mut ConfObject,
    val: *mut attr_value_t,
    cb: *mut c_void,
) -> SetErr
where
    F: FnMut(*mut ConfObject, AttrValue) -> Result<SetErr> + 'static,
{
    let closure = Box::leak(unsafe { Box::from_raw(cb as *mut Box<F>) });
    let val = unsafe { AttrValue::from_raw(val) };

    closure(obj, val).expect("Error calling set_attr_handler callback")
}

extern "C" fn get_class_attr_handler<F>(cls: *mut ConfClass, cb: *mut c_void) -> attr_value_t
where
    F: FnMut(*mut ConfClass) -> Result<AttrValue> + 'static,
{
    let closure = Box::leak(unsafe { Box::from_raw(cb as *mut Box<F>) });

    closure(cls)
        .expect("Error calling get_class_attr_handler callback")
        .into_raw()
}

extern "C" fn set_class_attr_handler<F>(
    cls: *mut ConfClass,
    val: *mut attr_value_t,
    cb: *mut c_void,
) -> SetErr
where
    F: FnMut(*mut ConfClass, AttrValue) -> Result<SetErr> + 'static,
{
    let closure = Box::leak(unsafe { Box::from_raw(cb as *mut Box<F>) });
    let val = unsafe { AttrValue::from_raw(val) };

    closure(cls, val).expect("Error calling set_class_attr_handler callback")
}

#[simics_exception]
/// Add the attribute name to the set of attributes of the class cls. This attribute
/// will appear on all instances of the class.
///
/// The function `getter` is called with the object and the value from user_data_get as
/// arguments, and returns the current value of the attribute.
///
/// On error, get_attr should call [`attribute_error`]. The return value is then
/// ignored; typically, [`make_attr_invalid`] is used to generate an explicitly invalid
/// value.
///
/// If `getter` is `None`, the attribute will be write-only. The function `setter` is
/// called with the object and the value from `user_data_set` as arguments when the
/// attribute is initialised or changed. The argument value is owned by the caller, so
/// any data from it must be copied.
///
/// The set_attr method should return [`SetErr::Sim_Set_Ok`] if the new value could be
/// set. On error, it should return an appropriate error code (usually
/// [`SetErr::Sim_Set_Illegal_Value`]), and optionally call [`attribute_error`] with an
/// explanatory message.
///
/// If setter is `None`, the attribute will be read-only.  The attr parameter
/// is one of [`AttrAttr::Sim_Attr_Required`], [`AttrAttr::Sim_Attr_Optional`],
/// [`AttrAttr::Sim_Attr_Session`] or [`AttrAttr::Sim_Attr_Pseudo`].  Attributes marked
/// [`AttrAttr::Sim_Attr_Required`] or [`AttrAttr::Sim_Attr_Optional`] are saved in
/// checkpoints.  Both `setter` and `getter` must be provided for such attributes.  All
/// attributes that are marked [`AttrAttr::Sim_Attr_Required`] must be present in all
/// configurations.
///
/// The set of permitted values is encoded in the `attr_type` type, and in `idx_type`
/// for values during indexed access. A `None` value for either type string means that
/// values of any type are permitted.
///
/// # Arguments
///
/// * `cls` - The class to register an attribute on
/// * `name` - The name of the new attribute
/// * `getter` - A closure that takes an instance of the object described by `cls` and
/// the value to get
/// * `setter` - An optional closure that takes an instance of the object described by
/// `cls`, the value to set, and the value to set it to
/// * `attr` - The attributes of the attribute
/// * `attr_type` - The types allowed for the attribute
/// * `idx_type` - The types allowed to index the attribute
///
/// # Context
///
/// Global Context
pub fn register_typed_attribute<S, GF, SF>(
    cls: *mut ConfClass,
    name: S,
    getter: Option<GF>,
    setter: Option<SF>,
    attr: AttrAttr,
    attr_type: Option<TypeStringType>,
    idx_type: Option<TypeStringType>,
    desc: S,
) -> Result<()>
where
    S: AsRef<str>,
    GF: FnMut(*mut ConfObject, AttrValue) -> Result<AttrValue> + 'static,
    SF: FnMut(*mut ConfObject, AttrValue, AttrValue) -> Result<SetErr> + 'static,
{
    let attr_type = if let Some(attr_type) = attr_type {
        raw_cstr(attr_type.to_string())?
    } else {
        null_mut()
    };

    let idx_type = if let Some(idx_type) = idx_type {
        raw_cstr(idx_type.to_string())?
    } else {
        null_mut()
    };

    let (get_attr, getter_cb_raw) = if let Some(getter) = getter {
        let getter_cb = Box::new(getter);
        let getter_cb_box = Box::new(getter_cb);
        (
            Some(get_typed_attr_handler::<GF> as _),
            Box::into_raw(getter_cb_box),
        )
    } else {
        (None, null_mut())
    };

    let (set_attr, setter_cb_raw) = if let Some(setter) = setter {
        let setter_cb = Box::new(setter);
        let setter_cb_box = Box::new(setter_cb);
        (
            Some(set_typed_attr_handler::<SF> as _),
            Box::into_raw(setter_cb_box),
        )
    } else {
        (None, null_mut())
    };

    unsafe {
        SIM_register_typed_attribute(
            cls,
            raw_cstr(name)?,
            get_attr,
            getter_cb_raw as *mut c_void,
            set_attr,
            setter_cb_raw as *mut c_void,
            attr,
            attr_type,
            idx_type,
            raw_cstr(desc)?,
        )
    };

    Ok(())
}

#[simics_exception]
/// Register a typed attribute of a class. This attribute will appear on the class object itself
/// and is the same for all instances of the class.
///
/// Add the attribute name to the set of attributes of the class cls.
///
/// The function `getter` is called with the object and the value from user_data_get as
/// arguments, and returns the current value of the attribute.
///
/// On error, get_attr should call [`attribute_error`]. The return value is then
/// ignored; typically, [`make_attr_invalid`] is used to generate an explicitly invalid
/// value.
///
/// If `getter` is `None`, the attribute will be write-only. The function `setter` is
/// called with the object and the value from `user_data_set` as arguments when the
/// attribute is initialised or changed. The argument value is owned by the caller, so
/// any data from it must be copied.
///
/// The set_attr method should return [`SetErr::Sim_Set_Ok`] if the new value could be
/// set. On error, it should return an appropriate error code (usually
/// [`SetErr::Sim_Set_Illegal_Value`]), and optionally call [`attribute_error`] with an
/// explanatory message.
///
/// If setter is `None`, the attribute will be read-only.  The attr parameter
/// is one of [`AttrAttr::Sim_Attr_Required`], [`AttrAttr::Sim_Attr_Optional`],
/// [`AttrAttr::Sim_Attr_Session`] or [`AttrAttr::Sim_Attr_Pseudo`].  Attributes marked
/// [`AttrAttr::Sim_Attr_Required`] or [`AttrAttr::Sim_Attr_Optional`] are saved in
/// checkpoints.  Both `setter` and `getter` must be provided for such attributes.  All
/// attributes that are marked [`AttrAttr::Sim_Attr_Required`] must be present in all
/// configurations.
///
/// The set of permitted values is encoded in the `attr_type` type, and in `idx_type`
/// for values during indexed access. A `None` value for either type string means that
/// values of any type are permitted.
///
/// # Arguments
///
/// * `cls` - The class to register an attribute on
/// * `name` - The name of the new attribute
/// * `getter` - A closure that takes an instance of the object described by `cls` and
/// the value to get
/// * `setter` - An optional closure that takes an instance of the object described by
/// `cls`, the value to set, and the value to set it to
/// * `attr` - The attributes of the attribute
/// * `attr_type` - The types allowed for the attribute
/// * `idx_type` - The types allowed to index the attribute
///
/// # Context
///
/// Global Context
pub fn register_typed_class_attribute<S, GF, SF>(
    cls: *mut ConfClass,
    name: S,
    getter: Option<GF>,
    setter: Option<SF>,
    attr: AttrAttr,
    attr_type: Option<TypeStringType>,
    idx_type: Option<TypeStringType>,
    desc: S,
) -> Result<()>
where
    S: AsRef<str>,
    GF: FnMut(*mut ConfClass, AttrValue) -> Result<AttrValue> + 'static,
    SF: FnMut(*mut ConfClass, AttrValue, AttrValue) -> Result<SetErr> + 'static,
{
    let attr_type = if let Some(attr_type) = attr_type {
        raw_cstr(attr_type.to_string())?
    } else {
        null_mut()
    };

    let idx_type = if let Some(idx_type) = idx_type {
        raw_cstr(idx_type.to_string())?
    } else {
        null_mut()
    };

    let (get_attr, getter_cb_raw) = if let Some(getter) = getter {
        let getter_cb = Box::new(getter);
        let getter_cb_box = Box::new(getter_cb);
        (
            Some(get_typed_class_attr_handler::<GF> as _),
            Box::into_raw(getter_cb_box),
        )
    } else {
        (None, null_mut())
    };

    let (set_attr, setter_cb_raw) = if let Some(setter) = setter {
        let setter_cb = Box::new(setter);
        let setter_cb_box = Box::new(setter_cb);
        (
            Some(set_typed_class_attr_handler::<SF> as _),
            Box::into_raw(setter_cb_box),
        )
    } else {
        (None, null_mut())
    };

    unsafe {
        SIM_register_typed_class_attribute(
            cls,
            raw_cstr(name)?,
            get_attr,
            getter_cb_raw as *mut c_void,
            set_attr,
            setter_cb_raw as *mut c_void,
            attr,
            attr_type,
            idx_type,
            raw_cstr(desc)?,
        )
    };

    Ok(())
}

#[simics_exception]
/// Register a pseudo-untyped attribute of the instances of a class.
///
/// Add the attribute name to the set of attributes of the class cls.
///
/// The function `getter` is called with the object and the value from user_data_get as
/// arguments, and returns the current value of the attribute.
///
/// On error, get_attr should call [`attribute_error`]. The return value is then
/// ignored; typically, [`make_attr_invalid`] is used to generate an explicitly invalid
/// value.
///
/// If `getter` is `None`, the attribute will be write-only. The function `setter` is
/// called with the object and the value from `user_data_set` as arguments when the
/// attribute is initialised or changed. The argument value is owned by the caller, so
/// any data from it must be copied.
///
/// The set_attr method should return [`SetErr::Sim_Set_Ok`] if the new value could be
/// set. On error, it should return an appropriate error code (usually
/// [`SetErr::Sim_Set_Illegal_Value`]), and optionally call [`attribute_error`] with an
/// explanatory message.
///
/// If setter is `None`, the attribute will be read-only.  The attr parameter
/// is one of [`AttrAttr::Sim_Attr_Required`], [`AttrAttr::Sim_Attr_Optional`],
/// [`AttrAttr::Sim_Attr_Session`] or [`AttrAttr::Sim_Attr_Pseudo`].  Attributes marked
/// [`AttrAttr::Sim_Attr_Required`] or [`AttrAttr::Sim_Attr_Optional`] are saved in
/// checkpoints.  Both `setter` and `getter` must be provided for such attributes.  All
/// attributes that are marked [`AttrAttr::Sim_Attr_Required`] must be present in all
/// configurations.
///
/// The set of permitted values is encoded in the `attr_type` type, and in `idx_type`
/// for values during indexed access. A `None` value for either type string means that
/// values of any type are permitted.
///
/// # Arguments
///
/// * `cls` - The class to register an attribute on
/// * `name` - The name of the new attribute
/// * `getter` - A closure that takes an instance of the object described by `cls` and
/// the value to get
/// * `setter` - An optional closure that takes an instance of the object described by
/// `cls`, the value to set, and the value to set it to
/// * `attr` - The attributes of the attribute
/// * `attr_type` - The types allowed for the attribute
///
/// # Context
///
/// Global Context
pub fn register_attribute<S, GF, SF>(
    cls: *mut ConfClass,
    name: S,
    getter: Option<GF>,
    setter: Option<SF>,
    attr: AttrAttr,
    attr_type: Option<TypeStringType>,
    desc: S,
) -> Result<()>
where
    S: AsRef<str>,
    GF: FnMut(*mut ConfObject) -> Result<AttrValue> + 'static,
    SF: FnMut(*mut ConfObject, AttrValue) -> Result<SetErr> + 'static,
{
    let attr_type = if let Some(attr_type) = attr_type {
        raw_cstr(attr_type.to_string())?
    } else {
        null_mut()
    };

    let (get_attr, getter_cb_raw) = if let Some(getter) = getter {
        let getter_cb = Box::new(getter);
        let getter_cb_box = Box::new(getter_cb);
        (
            Some(get_attr_handler::<GF> as _),
            Box::into_raw(getter_cb_box),
        )
    } else {
        (None, null_mut())
    };

    let (set_attr, setter_cb_raw) = if let Some(setter) = setter {
        let setter_cb = Box::new(setter);
        let setter_cb_box = Box::new(setter_cb);
        (
            Some(set_attr_handler::<SF> as _),
            Box::into_raw(setter_cb_box),
        )
    } else {
        (None, null_mut())
    };

    unsafe {
        SIM_register_attribute_with_user_data(
            cls,
            raw_cstr(name)?,
            get_attr,
            getter_cb_raw as *mut c_void,
            set_attr,
            setter_cb_raw as *mut c_void,
            attr,
            attr_type,
            raw_cstr(desc)?,
        )
    };

    Ok(())
}

#[simics_exception]
/// Register a pseudo-untyped attribute on a class itself.
///
/// Add the attribute name to the set of attributes of the class cls.
///
/// The function `getter` is called with the object and the value from user_data_get as
/// arguments, and returns the current value of the attribute.
///
/// On error, get_attr should call [`attribute_error`]. The return value is then
/// ignored; typically, [`make_attr_invalid`] is used to generate an explicitly invalid
/// value.
///
/// If `getter` is `None`, the attribute will be write-only. The function `setter` is
/// called with the object and the value from `user_data_set` as arguments when the
/// attribute is initialised or changed. The argument value is owned by the caller, so
/// any data from it must be copied.
///
/// The set_attr method should return [`SetErr::Sim_Set_Ok`] if the new value could be
/// set. On error, it should return an appropriate error code (usually
/// [`SetErr::Sim_Set_Illegal_Value`]), and optionally call [`attribute_error`] with an
/// explanatory message.
///
/// If setter is `None`, the attribute will be read-only.  The attr parameter
/// is one of [`AttrAttr::Sim_Attr_Required`], [`AttrAttr::Sim_Attr_Optional`],
/// [`AttrAttr::Sim_Attr_Session`] or [`AttrAttr::Sim_Attr_Pseudo`].  Attributes marked
/// [`AttrAttr::Sim_Attr_Required`] or [`AttrAttr::Sim_Attr_Optional`] are saved in
/// checkpoints.  Both `setter` and `getter` must be provided for such attributes.  All
/// attributes that are marked [`AttrAttr::Sim_Attr_Required`] must be present in all
/// configurations.
///
/// The set of permitted values is encoded in the `attr_type` type, and in `idx_type`
/// for values during indexed access. A `None` value for either type string means that
/// values of any type are permitted.
///
/// # Arguments
///
/// * `cls` - The class to register an attribute on
/// * `name` - The name of the new attribute
/// * `getter` - A closure that takes an instance of the object described by `cls` and
/// the value to get
/// * `setter` - An optional closure that takes an instance of the object described by
/// `cls`, the value to set, and the value to set it to
/// * `attr` - The attributes of the attribute
/// * `attr_type` - The types allowed for the attribute
///
/// # Context
///
/// Global Context
pub fn register_class_attribute<S, GF, SF>(
    cls: *mut ConfClass,
    name: S,
    getter: Option<GF>,
    setter: Option<SF>,
    attr: AttrAttr,
    attr_type: Option<TypeStringType>,
    desc: S,
) -> Result<()>
where
    S: AsRef<str>,
    GF: FnMut(*mut ConfClass) -> Result<AttrValue> + 'static,
    SF: FnMut(*mut ConfClass, AttrValue) -> Result<SetErr> + 'static,
{
    let attr_type = if let Some(attr_type) = attr_type {
        raw_cstr(attr_type.to_string())?
    } else {
        null_mut()
    };

    let (get_attr, getter_cb_raw) = if let Some(getter) = getter {
        let getter_cb = Box::new(getter);
        let getter_cb_box = Box::new(getter_cb);
        (
            Some(get_class_attr_handler::<GF> as _),
            Box::into_raw(getter_cb_box),
        )
    } else {
        (None, null_mut())
    };

    let (set_attr, setter_cb_raw) = if let Some(setter) = setter {
        let setter_cb = Box::new(setter);
        let setter_cb_box = Box::new(setter_cb);
        (
            Some(set_class_attr_handler::<SF> as _),
            Box::into_raw(setter_cb_box),
        )
    } else {
        (None, null_mut())
    };

    unsafe {
        SIM_register_class_attribute_with_user_data(
            cls,
            raw_cstr(name)?,
            get_attr,
            getter_cb_raw as *mut c_void,
            set_attr,
            setter_cb_raw as *mut c_void,
            attr,
            attr_type,
            raw_cstr(desc)?,
        )
    };

    Ok(())
}

// NOTE: We do not provide unuserdata untyped registration functions, we only want to register
// typed attributes, and we need userdata for our handlers

#[simics_exception]
/// When used inside an attribute set_attr/get_attr method, indicates why it failed to
/// set or retrieve the attribute. This function only serves to give an informative
/// message to the user. The object or attribute names need not be mentioned in the msg
/// argument; Simics will supply this automatically.
///
/// The error message supplied will be attached to any frontend exception generated by
/// the attribute access.
///
/// # Arguments
///
/// * `msg` - The message to set on an attribute error
///
/// # Context
///
/// Cell Context
pub fn attribute_error<S>(msg: S) -> Result<()>
where
    S: AsRef<str>,
{
    unsafe { SIM_attribute_error(raw_cstr(msg)?) };
    Ok(())
}

// NOTE: add_configuration not implemented, it is only to be used from Python

#[simics_exception]
/// Register that cls implements interface `I`. The interface itself should be
/// supplied in the iface argument.
///
/// The data iface points to must not be deallocated or overwritten by the caller.
/// Simics will use that data to store the interface structure. It will never be freed
/// or written to by Simics.
///
/// # Arguments
///
/// * `cls` - The class to register the interface for
///
/// # Return value
///
/// Non-zero on failure, 0 on success
///
/// # Exceptions
///
/// * [`SimException::SimExc_General`] Thrown if the interface name is illegal, or if
/// this interface has already been registered for this class.
///
/// # Context
///
/// Global Context
pub fn register_interface<I>(cls: *mut ConfClass) -> Result<i32>
where
    I: Interface,
{
    let name_raw = I::NAME.as_raw_cstr()?;
    let iface_box = Box::<I::InternalInterface>::default();
    // Note: This allocates and never frees. This is *required* by SIMICS and it is an error to
    // free this pointer
    let iface_raw = Box::into_raw(iface_box);

    debug_assert!(
        std::mem::size_of_val(&iface_raw) == std::mem::size_of::<*mut std::ffi::c_void>(),
        "Pointer is not convertible to *mut c_void"
    );

    Ok(unsafe { SIM_register_interface(cls, name_raw, iface_raw as *mut _) })
}

// TODO: Port & compatible interfaces

#[simics_exception]
/// Get an interface on an object
///
/// # Arguments
///
/// * `obj` - The object to get an interface on
///
/// # Return Value
///
/// The interface requested, or an error if invalid.
///
/// # Performance
///
/// * `SIM_get_interface` - Calls [`SIM_object_class`] which is extremely cheap ((&obj->sobj)->isa)
///   then canonicalizes (replace - with _) the interface name, then does a hashtable lookup with
///   the interface name. This shouldn't be called in an extreme tight loop (e.g. each instruction)
///   but is OK to call on rarer events (e.g. on magic instructions).
///
/// # Context
///
/// All Contexts
pub fn get_interface<I>(obj: *mut ConfObject) -> Result<I>
where
    I: Interface,
{
    Ok(I::new(obj, unsafe {
        SIM_get_interface(obj as *const ConfObject, I::NAME.as_raw_cstr()?)
            as *mut I::InternalInterface
    }))
}

#[simics_exception]
/// Get an interface of a class
///
/// # Arguments
///
/// * `obj` - The object to get an interface on
///
/// # Return Value
///
/// The interface requested, or an error if invalid.
///
/// # Context
///
/// All Contexts
pub fn get_class_interface<I>(cls: *mut ConfClass) -> Result<*mut I::InternalInterface>
where
    I: Interface,
{
    Ok(unsafe {
        SIM_get_class_interface(cls as *const ConfClass, I::NAME.as_raw_cstr()?)
            as *mut I::InternalInterface
    })
}

// TODO: Add Port Interfaces

#[simics_exception]
/// Indicates if the given object is being deleted. This information can be useful by
/// other objects that want to clean up their references.
///
/// # Return Value
///
/// Whether the object is being deleted
///
/// # Context
///
/// Global Context
pub fn marked_for_deletion(obj: *mut ConfObject) -> bool {
    unsafe { SIM_marked_for_deletion(obj) }
}
