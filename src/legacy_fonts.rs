//! Original DirectWrite font loaders. Own font bytes for as long as DirectWrite
//! references them, including fonts added by GPUI components at runtime.
use parking_lot::RwLock;
use std::{borrow::Cow, ffi::c_void, sync::Arc};
use windows::{
    Win32::{Foundation::*, Graphics::DirectWrite::*},
    core::*,
};

type FontBytes = Arc<RwLock<Vec<Arc<[u8]>>>>;

pub(super) struct FontLoader {
    factory: IDWriteFactory,
    data: FontBytes,
    file_loader: IDWriteFontFileLoader,
    collection_loader: IDWriteFontCollectionLoader,
}

impl FontLoader {
    pub fn new(factory: &IDWriteFactory) -> Result<Self> {
        let data = Arc::new(RwLock::new(Vec::new()));
        let file_loader: IDWriteFontFileLoader = FileLoader { data: data.clone() }.into();
        let collection_loader: IDWriteFontCollectionLoader = CollectionLoader {
            file_loader: file_loader.clone(),
        }
        .into();
        unsafe {
            factory.RegisterFontFileLoader(&file_loader)?;
            if let Err(error) = factory.RegisterFontCollectionLoader(&collection_loader) {
                let _ = factory.UnregisterFontFileLoader(&file_loader);
                return Err(error);
            }
        }
        Ok(Self {
            factory: factory.clone(),
            data,
            file_loader,
            collection_loader,
        })
    }

    pub fn add(&self, fonts: Vec<Cow<'static, [u8]>>) {
        self.data.write().extend(
            fonts
                .into_iter()
                .map(|data| Arc::<[u8]>::from(data.as_ref())),
        );
    }

    pub fn collection(&self, factory: &IDWriteFactory) -> Result<IDWriteFontCollection> {
        let count = self.data.read().len();
        unsafe {
            factory.CreateCustomFontCollection(
                &self.collection_loader,
                (&count as *const usize).cast(),
                std::mem::size_of::<usize>() as u32,
            )
        }
    }
}

impl Drop for FontLoader {
    fn drop(&mut self) {
        unsafe {
            let _ = self
                .factory
                .UnregisterFontCollectionLoader(&self.collection_loader);
            let _ = self.factory.UnregisterFontFileLoader(&self.file_loader);
        }
    }
}

unsafe fn read_key(key: *const c_void, size: u32) -> Result<usize> {
    if key.is_null() || size as usize != std::mem::size_of::<usize>() {
        return Err(Error::from_hresult(E_INVALIDARG));
    }
    Ok(unsafe { std::ptr::read_unaligned(key.cast()) })
}

#[implement(IDWriteFontFileLoader)]
struct FileLoader {
    data: FontBytes,
}

#[allow(non_snake_case)]
impl IDWriteFontFileLoader_Impl for FileLoader_Impl {
    fn CreateStreamFromKey(&self, key: *const c_void, size: u32) -> Result<IDWriteFontFileStream> {
        let index = unsafe { read_key(key, size)? };
        let data = self
            .data
            .read()
            .get(index)
            .cloned()
            .ok_or_else(|| Error::from_hresult(E_INVALIDARG))?;
        Ok(FontStream { data }.into())
    }
}

#[implement(IDWriteFontFileStream)]
struct FontStream {
    data: Arc<[u8]>,
}

#[allow(non_snake_case)]
impl IDWriteFontFileStream_Impl for FontStream_Impl {
    fn ReadFileFragment(
        &self,
        start: *mut *mut c_void,
        offset: u64,
        size: u64,
        context: *mut *mut c_void,
    ) -> Result<()> {
        let end = offset
            .checked_add(size)
            .ok_or_else(|| Error::from_hresult(E_INVALIDARG))?;
        if start.is_null() || context.is_null() || end > self.data.len() as u64 {
            return Err(Error::from_hresult(E_INVALIDARG));
        }
        unsafe {
            *start = self.data.as_ptr().add(offset as usize).cast_mut().cast();
            *context = std::ptr::null_mut();
        }
        Ok(())
    }
    fn ReleaseFileFragment(&self, _: *mut c_void) {}
    fn GetFileSize(&self) -> Result<u64> {
        Ok(self.data.len() as u64)
    }
    fn GetLastWriteTime(&self) -> Result<u64> {
        Ok(0)
    }
}

#[implement(IDWriteFontCollectionLoader)]
struct CollectionLoader {
    file_loader: IDWriteFontFileLoader,
}

#[allow(non_snake_case)]
impl IDWriteFontCollectionLoader_Impl for CollectionLoader_Impl {
    fn CreateEnumeratorFromKey(
        &self,
        factory: Ref<'_, IDWriteFactory>,
        key: *const c_void,
        size: u32,
    ) -> Result<IDWriteFontFileEnumerator> {
        let count = unsafe { read_key(key, size)? };
        let factory = factory
            .as_ref()
            .ok_or_else(|| Error::from_hresult(E_INVALIDARG))?;
        let mut files = Vec::with_capacity(count);
        for index in 0..count {
            files.push(unsafe {
                factory.CreateCustomFontFileReference(
                    (&index as *const usize).cast(),
                    std::mem::size_of::<usize>() as u32,
                    &self.file_loader,
                )?
            });
        }
        Ok(FontEnumerator {
            files,
            index: parking_lot::Mutex::new(None),
        }
        .into())
    }
}

#[implement(IDWriteFontFileEnumerator)]
struct FontEnumerator {
    files: Vec<IDWriteFontFile>,
    index: parking_lot::Mutex<Option<usize>>,
}

#[allow(non_snake_case)]
impl IDWriteFontFileEnumerator_Impl for FontEnumerator_Impl {
    fn MoveNext(&self) -> Result<BOOL> {
        let mut index = self.index.lock();
        let next = index.map_or(0, |index| index.saturating_add(1));
        *index = Some(next);
        Ok(BOOL::from(next < self.files.len()))
    }
    fn GetCurrentFontFile(&self) -> Result<IDWriteFontFile> {
        self.index
            .lock()
            .and_then(|index| self.files.get(index).cloned())
            .ok_or_else(|| Error::from_hresult(E_FAIL))
    }
}
