#[repr(C)]
pub struct TestModule {
    conf_object: simics_api::ConfObject,
}
#[no_mangle]
pub extern "C" fn testmodule_alloc(cls: *mut simics_api::ConfClass) -> *mut simics_api::ConfObject {
    let cls: *mut simics_api::ConfClass = cls.into();
    let obj: *mut simics_api::ConfObject = TestModule::alloc::<TestModule>(cls)
        .unwrap_or_else(|e| panic!("{}::alloc failed: {}", "testmodule", e))
        .into();
    obj
}
#[no_mangle]
pub extern "C" fn testmodule_init(obj: *mut simics_api::ConfObject) -> *mut std::ffi::c_void {
    let ptr: *mut ConfObject = TestModule::init(obj.into())
        .unwrap_or_else(|e| panic!("{}::init failed: {}", "testmodule", e))
        .into();
    ptr as *mut std::ffi::c_void
}
#[no_mangle]
pub extern "C" fn testmodule_finalize(obj: *mut simics_api::ConfObject) {
    TestModule::finalize(obj.into())
        .unwrap_or_else(|e| panic!("{}::finalize failed: {}", "testmodule", e));
}
#[no_mangle]
pub extern "C" fn testmodule_objects_finalized(obj: *mut simics_api::ConfObject) {
    TestModule::objects_finalized(obj.into())
        .unwrap_or_else(|e| panic!("{}::objects_finalized failed: {}", "testmodule", e));
}
#[no_mangle]
pub extern "C" fn testmodule_deinit(obj: *mut simics_api::ConfObject) {
    TestModule::deinit(obj.into())
        .unwrap_or_else(|e| panic!("{}::deinit failed: {}", "testmodule", e));
}
#[no_mangle]
pub extern "C" fn testmodule_dealloc(obj: *mut simics_api::ConfObject) {
    TestModule::dealloc(obj.into())
        .unwrap_or_else(|e| panic!("{}::dealloc failed: {}", "testmodule", e));
}
impl TestModule {
    const CLASS: simics_api::ClassInfo = simics_api::ClassInfo {
        alloc: Some(testmodule_alloc),
        init: Some(testmodule_init),
        finalize: Some(testmodule_finalize),
        objects_finalized: Some(testmodule_objects_finalized),
        deinit: Some(testmodule_deinit),
        dealloc: Some(testmodule_dealloc),
        description: raw_cstr::c_str!("testmodule").as_ptr(),
        short_desc: raw_cstr::c_str!("testmodule").as_ptr(),
        kind: simics_api::ClassKind::Vanilla as u32,
    };
}
impl simics_api::Create for TestModule {
    fn create() -> anyhow::Result<*mut simics_api::ConfClass> {
        simics_api::create_class("test_module", TestModule::CLASS)
    }
}
impl TestModule {
    fn new(obj: *mut simics_api::ConfObject) -> *mut simics_api::ConfObject {
        let obj_ptr: *mut simics_api::ConfObject = obj.into();
        let ptr: *mut TestModule = obj_ptr as *mut TestModule;
        (ptr as *mut simics_api::ConfObject).into()
    }
}
impl From<*mut simics_api::ConfObject> for &mut TestModule {
    fn from(value: *mut simics_api::ConfObject) -> Self {
        let ptr: *mut TestModule = value as *mut TestModule;
        unsafe { &mut *ptr }
    }
}
#[derive(Module)]
#[repr(C)]
pub struct TestModule3 {
    conf_object: simics_api::ConfObject,
}
#[no_mangle]
pub extern "C" fn testmodule3_alloc(
    cls: *mut simics_api::ConfClass,
) -> *mut simics_api::ConfObject {
    let cls: *mut simics_api::ConfClass = cls.into();
    let obj: *mut simics_api::ConfObject = TestModule3::alloc::<TestModule3>(cls)
        .unwrap_or_else(|e| panic!("{}::alloc failed: {}", "testmodule3", e))
        .into();
    obj
}
#[no_mangle]
pub extern "C" fn testmodule3_init(obj: *mut simics_api::ConfObject) -> *mut std::ffi::c_void {
    let ptr: *mut ConfObject = TestModule3::init(obj.into())
        .unwrap_or_else(|e| panic!("{}::init failed: {}", "testmodule3", e))
        .into();
    ptr as *mut std::ffi::c_void
}
#[no_mangle]
pub extern "C" fn testmodule3_finalize(obj: *mut simics_api::ConfObject) {
    TestModule3::finalize(obj.into())
        .unwrap_or_else(|e| panic!("{}::finalize failed: {}", "testmodule3", e));
}
#[no_mangle]
pub extern "C" fn testmodule3_objects_finalized(obj: *mut simics_api::ConfObject) {
    TestModule3::objects_finalized(obj.into())
        .unwrap_or_else(|e| panic!("{}::objects_finalized failed: {}", "testmodule3", e));
}
#[no_mangle]
pub extern "C" fn testmodule3_deinit(obj: *mut simics_api::ConfObject) {
    TestModule3::deinit(obj.into())
        .unwrap_or_else(|e| panic!("{}::deinit failed: {}", "testmodule3", e));
}
#[no_mangle]
pub extern "C" fn testmodule3_dealloc(obj: *mut simics_api::ConfObject) {
    TestModule3::dealloc(obj.into())
        .unwrap_or_else(|e| panic!("{}::dealloc failed: {}", "testmodule3", e));
}
impl TestModule3 {
    const CLASS: simics_api::ClassInfo = simics_api::ClassInfo {
        alloc: Some(testmodule3_alloc),
        init: Some(testmodule3_init),
        finalize: Some(testmodule3_finalize),
        objects_finalized: Some(testmodule3_objects_finalized),
        deinit: Some(testmodule3_deinit),
        dealloc: Some(testmodule3_dealloc),
        description: raw_cstr::c_str!("testmodule3").as_ptr(),
        short_desc: raw_cstr::c_str!("testmodule3").as_ptr(),
        kind: simics_api::ClassKind::Vanilla as u32,
    };
}
impl simics_api::Create for TestModule3 {
    fn create() -> anyhow::Result<*mut simics_api::ConfClass> {
        simics_api::create_class("test_module_3", TestModule3::CLASS)
    }
}
impl TestModule3 {
    fn new(obj: *mut simics_api::ConfObject) -> *mut simics_api::ConfObject {
        let obj_ptr: *mut simics_api::ConfObject = obj.into();
        let ptr: *mut TestModule3 = obj_ptr as *mut TestModule3;
        (ptr as *mut simics_api::ConfObject).into()
    }
}
impl From<*mut simics_api::ConfObject> for &mut TestModule3 {
    fn from(value: *mut simics_api::ConfObject) -> Self {
        let ptr: *mut TestModule3 = value as *mut TestModule3;
        unsafe { &mut *ptr }
    }
}
#[derive(Module)]
#[repr(C)]
pub struct TestModule4 {
    conf_object: simics_api::ConfObject,
}
#[no_mangle]
pub extern "C" fn testmodule4_alloc(
    cls: *mut simics_api::ConfClass,
) -> *mut simics_api::ConfObject {
    let cls: *mut simics_api::ConfClass = cls.into();
    let obj: *mut simics_api::ConfObject = TestModule4::alloc::<TestModule4>(cls)
        .unwrap_or_else(|e| panic!("{}::alloc failed: {}", "testmodule4", e))
        .into();
    obj
}
#[no_mangle]
pub extern "C" fn testmodule4_init(obj: *mut simics_api::ConfObject) -> *mut std::ffi::c_void {
    let ptr: *mut ConfObject = TestModule4::init(obj.into())
        .unwrap_or_else(|e| panic!("{}::init failed: {}", "testmodule4", e))
        .into();
    ptr as *mut std::ffi::c_void
}
#[no_mangle]
pub extern "C" fn testmodule4_finalize(obj: *mut simics_api::ConfObject) {
    TestModule4::finalize(obj.into())
        .unwrap_or_else(|e| panic!("{}::finalize failed: {}", "testmodule4", e));
}
#[no_mangle]
pub extern "C" fn testmodule4_objects_finalized(obj: *mut simics_api::ConfObject) {
    TestModule4::objects_finalized(obj.into())
        .unwrap_or_else(|e| panic!("{}::objects_finalized failed: {}", "testmodule4", e));
}
#[no_mangle]
pub extern "C" fn testmodule4_deinit(obj: *mut simics_api::ConfObject) {
    TestModule4::deinit(obj.into())
        .unwrap_or_else(|e| panic!("{}::deinit failed: {}", "testmodule4", e));
}
#[no_mangle]
pub extern "C" fn testmodule4_dealloc(obj: *mut simics_api::ConfObject) {
    TestModule4::dealloc(obj.into())
        .unwrap_or_else(|e| panic!("{}::dealloc failed: {}", "testmodule4", e));
}
impl TestModule4 {
    const CLASS: simics_api::ClassInfo = simics_api::ClassInfo {
        alloc: Some(testmodule4_alloc),
        init: Some(testmodule4_init),
        finalize: Some(testmodule4_finalize),
        objects_finalized: Some(testmodule4_objects_finalized),
        deinit: Some(testmodule4_deinit),
        dealloc: Some(testmodule4_dealloc),
        description: raw_cstr::c_str!("Test module 4").as_ptr(),
        short_desc: raw_cstr::c_str!("TM4").as_ptr(),
        kind: ClassKind::Session as u32,
    };
}
impl simics_api::Create for TestModule4 {
    fn create() -> anyhow::Result<*mut simics_api::ConfClass> {
        simics_api::create_class("test_module_4", TestModule4::CLASS)
    }
}
impl TestModule4 {
    fn new(obj: *mut simics_api::ConfObject) -> *mut simics_api::ConfObject {
        let obj_ptr: *mut simics_api::ConfObject = obj.into();
        let ptr: *mut TestModule4 = obj_ptr as *mut TestModule4;
        (ptr as *mut simics_api::ConfObject).into()
    }
}
impl From<*mut simics_api::ConfObject> for &mut TestModule4 {
    fn from(value: *mut simics_api::ConfObject) -> Self {
        let ptr: *mut TestModule4 = value as *mut TestModule4;
        unsafe { &mut *ptr }
    }
}
