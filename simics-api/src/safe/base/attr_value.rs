use crate::ConfObject;
use anyhow::{ensure, Context, Error, Result};
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::{FromPrimitive as ConvertFromPrimitive, ToPrimitive as ConvertToPrimitive};
use raw_cstr::raw_cstr;
use simics_api_sys::{
    attr_kind_t_Sim_Val_Boolean, attr_kind_t_Sim_Val_Data, attr_kind_t_Sim_Val_Dict,
    attr_kind_t_Sim_Val_Floating, attr_kind_t_Sim_Val_Integer, attr_kind_t_Sim_Val_Invalid,
    attr_kind_t_Sim_Val_List, attr_kind_t_Sim_Val_Nil, attr_kind_t_Sim_Val_Object,
    attr_kind_t_Sim_Val_String, attr_value__bindgen_ty_1, attr_value_t,
};
use std::{ffi::CStr, ptr::null_mut};

pub type AttrValue = attr_value_t;

#[derive(Debug, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum AttrKind {
    Boolean = attr_kind_t_Sim_Val_Boolean,
    Data = attr_kind_t_Sim_Val_Data,
    Dict = attr_kind_t_Sim_Val_Dict,
    Floating = attr_kind_t_Sim_Val_Floating,
    Integer = attr_kind_t_Sim_Val_Integer,
    Invalid = attr_kind_t_Sim_Val_Invalid,
    List = attr_kind_t_Sim_Val_List,
    Nil = attr_kind_t_Sim_Val_Nil,
    Object = attr_kind_t_Sim_Val_Object,
    String = attr_kind_t_Sim_Val_String,
}

impl TryFrom<AttrKind> for u32 {
    type Error = Error;

    fn try_from(value: AttrKind) -> Result<Self> {
        value.to_u32().context(format!("Invalid value {:?}", value))
    }
}

impl TryFrom<u32> for AttrKind {
    type Error = Error;

    fn try_from(value: u32) -> Result<Self> {
        ConvertFromPrimitive::from_u32(value).context(format!("Invalid value {}", value))
    }
}

pub fn make_attr_invalid() -> Result<AttrValue> {
    Ok(AttrValue {
        private_kind: AttrKind::Invalid.try_into()?,
        private_size: 0,
        private_u: attr_value__bindgen_ty_1 { integer: 0 },
    })
}

/* <add-fun id="device api attr_value_t">
  <short>make nil attribute</short>
  Returns an <type>attr_value_t</type> of type nil.

  <di name="EXECUTION CONTEXT">Cell Context</di>
</add-fun> */
pub fn make_attr_nil() -> Result<AttrValue> {
    Ok(AttrValue {
        private_kind: AttrKind::Nil.try_into()?,
        private_size: 0,
        private_u: attr_value__bindgen_ty_1 { integer: 0 },
    })
}

/* <append-fun id="SIM_make_attr_int64"></append-fun> */
pub fn make_attr_uint64(i: u64) -> Result<AttrValue> {
    Ok(AttrValue {
        private_kind: AttrKind::Integer.try_into()?,
        private_size: 0, /* unsigned */
        private_u: attr_value__bindgen_ty_1 {
            integer: i64::try_from(i)?,
        },
    })
}

/* <add-fun id="device api attr_value_t">
  <short>make integer attribute</short>
  Returns an <type>attr_value_t</type> of integer type with value
  <param>i</param>.

  <di name="EXECUTION CONTEXT">Cell Context</di>
</add-fun> */
pub fn make_attr_int64(i: i64) -> Result<AttrValue> {
    Ok(AttrValue {
        private_kind: AttrKind::Integer.try_into()?,
        private_size: 1, /* signed */
        private_u: attr_value__bindgen_ty_1 { integer: i },
    })
}

/* <add-fun id="device api attr_value_t">
  <short>make boolean attribute</short>
  Returns an <type>attr_value_t</type> of boolean type.

  <di name="EXECUTION CONTEXT">Cell Context</di>
</add-fun> */
pub fn make_attr_boolean(b: bool) -> Result<AttrValue> {
    Ok(AttrValue {
        private_kind: AttrKind::Boolean.try_into()?,
        private_size: 0,
        private_u: attr_value__bindgen_ty_1 { boolean: b },
    })
}

/* <append-fun id="SIM_make_attr_string"></append-fun> */
pub fn make_attr_string_adopt<S: AsRef<str>>(s: S) -> Result<AttrValue> {
    let string = raw_cstr(s)?;
    Ok(AttrValue {
        private_kind: if string.is_null() {
            AttrKind::Nil.try_into()?
        } else {
            AttrKind::String.try_into()?
        },
        private_size: 0,
        private_u: attr_value__bindgen_ty_1 { string },
    })
}

/* <add-fun id="device api attr_value_t">
  <short>make floating point attribute</short>
  Returns an <type>attr_value_t</type> of floating type with value
  <param>d</param>.

  <di name="EXECUTION CONTEXT">Cell Context</di>
</add-fun> */
pub fn make_attr_floating(d: f64) -> Result<AttrValue> {
    Ok(AttrValue {
        private_kind: AttrKind::Floating.try_into()?,
        private_size: 0,
        private_u: attr_value__bindgen_ty_1 { floating: d },
    })
}

/* <add-fun id="device api attr_value_t">
  <short>make object attribute</short>
  Returns an <type>attr_value_t</type> of object type
  with value <param>obj</param>. Returns a nil value if
  <param>obj</param> is <const>NULL</const>.

  <di name="EXECUTION CONTEXT">Cell Context</di>
</add-fun> */
pub fn make_attr_object(obj: ConfObject) -> Result<AttrValue> {
    Ok(AttrValue {
        private_kind: if obj.as_const().is_null() {
            AttrKind::Nil.try_into()?
        } else {
            AttrKind::Object.try_into()?
        },
        private_size: 0,
        private_u: attr_value__bindgen_ty_1 { object: obj.into() },
    })
}

/* <append-fun id="SIM_make_attr_data"></append-fun> */
pub fn make_attr_data_adopt<T>(size: usize, data: T) -> Result<AttrValue> {
    let data = Box::new(data);
    let data_ptr = Box::into_raw(data);
    ensure!(
        !data_ptr.is_null() && size == 0,
        "NULL data requires zero size"
    );
    Ok(attr_value_t {
        private_kind: AttrKind::Data.try_into()?,
        private_size: u32::try_from(size)?,
        private_u: attr_value__bindgen_ty_1 {
            data: data_ptr as *mut u8,
        },
    })
}

/* <append-fun id="SIM_attr_is_integer"/> */
pub fn attr_is_nil(attr: AttrValue) -> Result<bool> {
    Ok(attr.private_kind == AttrKind::Nil.try_into()?)
}

/* <append-fun id="SIM_attr_is_integer"/> */
pub fn attr_is_int64(attr: AttrValue) -> Result<bool> {
    Ok(attr.private_kind == AttrKind::Integer.try_into()?
        && (attr.private_size == 0 || unsafe { attr.private_u.integer } >= 0))
}

/* <append-fun id="SIM_attr_is_integer"/> */
pub fn attr_is_uint64(attr: AttrValue) -> Result<bool> {
    Ok(attr.private_kind == AttrKind::Integer.try_into()?
        && (attr.private_size != 0 || unsafe { attr.private_u.integer } >= 0))
}

/* <add-fun id="device api attr_value_t">
  <short><type>attr_value_t</type> type predicates</short>

  Indicates whether the value stored in <arg>attr</arg> is of the specified
  type. <fun>SIM_attr_is_int64</fun> and <fun>SIM_attr_is_uint64</fun>
  additionally test whether the integer value would fit in the given C type.

  <di name="EXECUTION CONTEXT">Cell Context</di>
</add-fun> */
pub fn attr_is_integer(attr: AttrValue) -> Result<bool> {
    Ok(attr.private_kind == AttrKind::Integer.try_into()?)
}

/* <add-fun id="device api attr_value_t">
  <short>extract values stored in <type>attr_value_t</type> values</short>

  Extract a value encapsulated in <param>attr</param>. It is an error to
  call an accessor function with an <param>attr</param> of the wrong type.

  <fun>SIM_attr_integer</fun> returns the integer attribute value
  modulo-reduced to the interval
  <math>[-2<sup>63</sup>,2<sup>63</sup>-1]</math>.
  (Converting the return value to <type>uint64</type> gives the integer
  attribute value modulo-reduced to <math>[0,2<sup>64</sup>-1]</math>.)

  <fun>SIM_attr_string()</fun>, <fun>SIM_attr_data()</fun> and
  <fun>SIM_attr_list()</fun> return values owned by <param>attr</param>.
  Ownership is not transferred to the caller.

  <fun>SIM_attr_string_detach()</fun> returns the string
  in <param>attr</param> and changes the value pointed to by
  <param>attr</param> into a nil attribute. Ownership of the string is
  transferred to the caller.

  <fun>SIM_attr_object_or_nil</fun> accepts an <param>attr</param> parameter
  of either object or nil type. In case of a nil attribute, the function
  returns NULL.

  <fun>SIM_attr_list_size()</fun> and <fun>SIM_attr_dict_size</fun> return
  the number of items in the list and key-value pairs in the dict
  respectively. <fun>SIM_attr_data_size()</fun> returns the number of bytes
  in the data value.

  <fun>SIM_attr_list_item()</fun> returns the item at <param>index</param>.
  The index must be less than the number of items in the list. The item
  returned is still owned by <param>attr</param>. Ownership is not
  transferred to the caller.

  <fun>SIM_attr_list()</fun> returns a pointer directly into the internal
  array of the attribute value; it is mainly present as an optimisation. Use
  <fun>SIM_attr_list_item()</fun> and <fun>SIM_attr_list_set_item()</fun>
  for type-safety instead.

  <fun>SIM_attr_dict_key()</fun> and <fun>SIM_attr_dict_value()</fun> return
  the key and value at <param>index</param>. The index must be less than the
  number of items in the dict. The value returned is still owned by
  <param>attr</param>. Ownership is not transferred to the caller.

  <di name="EXECUTION CONTEXT">
  All contexts (including Threaded Context)
  </di>

</add-fun> */
pub fn attr_integer(attr: AttrValue) -> Result<i64> {
    ensure!(attr_is_integer(attr)?, "Attribute must be integer!");
    Ok(unsafe { attr.private_u.integer })
}

/* <append-fun id="SIM_attr_is_integer"/> */
pub fn attr_is_boolean(attr: AttrValue) -> Result<bool> {
    Ok(attr.private_kind == AttrKind::Boolean.try_into()?)
}

/* <append-fun id="SIM_attr_integer"/> */
pub fn attr_boolean(attr: AttrValue) -> Result<bool> {
    ensure!(attr_is_boolean(attr)?, "Attribute must be bool!");
    Ok(unsafe { attr.private_u.boolean })
}

/* <append-fun id="SIM_attr_is_integer"/> */
pub fn attr_is_string(attr: AttrValue) -> Result<bool> {
    Ok(attr.private_kind == AttrKind::String.try_into()?)
}

/* <append-fun id="SIM_attr_integer"/> */
pub fn attr_string(attr: AttrValue) -> Result<String> {
    ensure!(attr_is_string(attr)?, "Attribute must be string!");
    Ok(unsafe { CStr::from_ptr(attr.private_u.string) }
        .to_string_lossy()
        .to_string())
}

/* <append-fun id="SIM_attr_integer"/> */
// TODO: Impl
// pub fn attr_string_detach(attr: *mut attr_value_t) -> char * {
//
//         char *ret;
//         VALIDATE_ATTR_KIND(SIM_attr_string_detach, *attr, String,
//                            (SIM_attr_free(attr),
//                             *attr = SIM_make_attr_nil(),
//                             MM_STRDUP("")));
//         ret = (char *)attr-.private_u.string;
//         *attr = SIM_make_attr_nil();
//         return ret;
// }

/* <append-fun id="SIM_attr_is_integer"/> */
pub fn attr_is_floating(attr: AttrValue) -> Result<bool> {
    Ok(attr.private_kind == AttrKind::Floating.try_into()?)
}

/* <append-fun id="SIM_attr_integer"/> */
pub fn attr_floating(attr: AttrValue) -> Result<f64> {
    ensure!(attr_is_floating(attr)?, "Attribute must be floating point!");
    Ok(unsafe { attr.private_u.floating })
}

/* <append-fun id="SIM_attr_is_integer"/> */
pub fn attr_is_object(attr: AttrValue) -> Result<bool> {
    Ok(attr.private_kind == AttrKind::Object.try_into()?)
}

/* <append-fun id="SIM_attr_integer"/> */
pub fn attr_object(attr: AttrValue) -> Result<ConfObject> {
    ensure!(attr_is_object(attr)?, "Attribute must be object!");
    Ok(ConfObject::new(unsafe { attr.private_u.object }))
}

/* <append-fun id="SIM_attr_integer"/> */
pub fn attr_object_or_nil(attr: AttrValue) -> Result<ConfObject> {
    if attr_is_nil(attr)? {
        Ok(ConfObject::new(null_mut()))
    } else {
        attr_object(attr)
    }
}

/* <append-fun id="SIM_attr_is_integer"/> */
pub fn attr_is_invalid(attr: AttrValue) -> Result<bool> {
    Ok(attr.private_kind == AttrKind::Invalid.try_into()?)
}

/* <append-fun id="SIM_attr_is_integer"/> */
pub fn attr_is_data(attr: AttrValue) -> Result<bool> {
    Ok(attr.private_kind == AttrKind::Data.try_into()?)
}

/* <append-fun id="SIM_attr_integer"/> */
pub fn attr_data_size(attr: AttrValue) -> Result<u32> {
    ensure!(attr_is_data(attr)?, "Attribute must be data!");
    Ok(attr.private_size)
}

/* <append-fun id="SIM_attr_integer"/> */
pub fn attr_data(attr: AttrValue) -> Result<Vec<u8>> {
    ensure!(attr_is_data(attr)?, "Attribute must be data!");
    Ok(unsafe {
        Vec::from_raw_parts(
            attr.private_u.data,
            attr_data_size(attr)? as usize,
            attr_data_size(attr)? as usize,
        )
    })
}

/* <append-fun id="SIM_attr_is_integer"/> */
pub fn attr_is_list(attr: AttrValue) -> Result<bool> {
    Ok(attr.private_kind == AttrKind::List.try_into()?)
}

/* <append-fun id="SIM_attr_integer"/> */
pub fn attr_list_size(attr: AttrValue) -> Result<u32> {
    ensure!(attr_is_list(attr)?, "Attribute must be list!");
    Ok(attr.private_size)
}

/* <append-fun id="SIM_attr_integer"/> */
/// Retrieve a list item from an attr
///
/// # Safety
///
/// The bounds of the list are checked before obtaining an offset, so this function will never
/// crash unless the list size is incorrectly set
pub unsafe fn attr_list_item(attr: AttrValue, index: u32) -> Result<AttrValue> {
    ensure!(attr_is_list(attr)?, "Attribute must be list!");
    ensure!(index < attr_list_size(attr)?, "Index out of bounds of list");
    Ok(unsafe { *attr.private_u.list.offset(index.try_into()?) })
}

/* <append-fun id="SIM_attr_integer"/> */
pub fn attr_list(attr: AttrValue) -> Result<*mut AttrValue> {
    ensure!(attr_is_list(attr)?, "Attribute must be list!");
    Ok(unsafe { attr.private_u.list })
}

/* <append-fun id="SIM_attr_is_integer"/> */
pub fn attr_is_dict(attr: AttrValue) -> bool {
    attr.private_kind == attr_kind_t_Sim_Val_Dict
}

/* <append-fun id="SIM_attr_integer"/> */
pub fn attr_dict_size(attr: AttrValue) -> Result<u32> {
    ensure!(attr_is_dict(attr), "Attribute must be dict!");
    Ok(attr.private_size)
}

/* <append-fun id="SIM_attr_integer"/> */
pub fn attr_dict_key(attr: AttrValue, index: u32) -> Result<AttrValue> {
    ensure!(attr_is_dict(attr), "Attribute must be dict!");
    ensure!(
        index < attr_dict_size(attr)?,
        "Index out of range of dictionary!"
    );
    let pair = unsafe { attr.private_u.dict.offset(index.try_into()?) };
    Ok(unsafe { *pair }.key)
}

/* <append-fun id="SIM_attr_integer"/> */
pub fn attr_dict_value(attr: AttrValue, index: u32) -> Result<AttrValue> {
    ensure!(attr_is_dict(attr), "Attribute must be dict!");
    ensure!(
        index < attr_dict_size(attr)?,
        "Index out of range of dictionary!"
    );
    let pair = unsafe { attr.private_u.dict.offset(index.try_into()?) };
    Ok(unsafe { *pair }.value)
}
