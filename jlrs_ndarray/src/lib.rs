//! Borrow data from Julia arrays as `ndarray`'s `ArrayView` and `ArrayViewMut`.
//!
//! This crate defines a single trait, `NdArray`, that provides methods that return an immutable
//! or a mutable view of the array data and is implemented by `Array` and `TypedArray` from jlrs.
//! It's easier to use this trait with `TypedArray`, you'll likely have to provide type
//! annotations with `Array`.

use jlrs::error::other;
use jlrs::prelude::*;
use ndarray::{ArrayView, ArrayViewMut, Dim, IntoDimension, IxDynImpl, ShapeBuilder};

mod private {
    use jlrs::traits::ValidLayout;
    use jlrs::value::array::{Array, TypedArray};

    pub trait Sealed {}
    impl<'frame, 'data> Sealed for Array<'frame, 'data> {}
    impl<'frame, 'data, T> Sealed for TypedArray<'frame, 'data, T> where T: Copy + ValidLayout {}
}

/// Trait to borrow Julia arrays with inline data as `ndarray`'s `ArrayView` and `ArrayViewMut`.
pub trait NdArray<'borrow, T>: private::Sealed {
    /// Borrow the data in the array as an `ArrayView`. Returns an error if the wrong type is
    /// provided or the data is not stored inline.
    fn array_view<'frame: 'borrow, F>(
        self,
        frame: &'borrow F,
    ) -> JlrsResult<ArrayView<'borrow, T, Dim<IxDynImpl>>>
    where
        F: Frame<'frame>,
        T: ValidLayout + Copy;

    /// Mutably borrow the data in the array as an `ArrayViewMut`. Returns an error if the wrong
    /// type is provided or the data is not stored inline.
    fn array_view_mut<'frame: 'borrow, F>(
        self,
        frame: &'borrow mut F,
    ) -> JlrsResult<ArrayViewMut<'borrow, T, Dim<IxDynImpl>>>
    where
        F: Frame<'frame>,
        T: ValidLayout + Copy;
}

impl<'frame: 'borrow, 'data: 'borrow, 'borrow, T: ValidLayout + Copy> NdArray<'borrow, T>
    for Array<'frame, 'data>
{
    fn array_view<'fr: 'borrow, F>(
        self,
        frame: &'borrow F,
    ) -> JlrsResult<ArrayView<'borrow, T, Dim<IxDynImpl>>>
    where
        F: Frame<'fr>,
        T: ValidLayout + Copy,
    {
        let data = self.inline_data::<T, _>(frame)?;
        let shape = data.dimensions().as_slice().into_dimension().f();
        match ArrayView::from_shape(shape, data.into_slice()) {
            Ok(arr) => Ok(arr),
            Err(e) => other(e)?,
        }
    }

    fn array_view_mut<'fr: 'borrow, F>(
        self,
        frame: &'borrow mut F,
    ) -> JlrsResult<ArrayViewMut<'borrow, T, Dim<IxDynImpl>>>
    where
        F: Frame<'fr>,
        T: ValidLayout + Copy,
    {
        let data = self.inline_data_mut::<T, _>(frame)?;
        let shape = data.dimensions().as_slice().into_dimension().f();
        let raw = data.into_mut_slice();
        match ArrayViewMut::from_shape(shape, raw) {
            Ok(arr) => Ok(arr),
            Err(e) => other(e)?,
        }
    }
}

impl<'frame: 'borrow, 'data: 'borrow, 'borrow, T: ValidLayout + Copy> NdArray<'borrow, T>
    for TypedArray<'frame, 'data, T>
{
    fn array_view<'fr: 'borrow, F>(
        self,
        frame: &'borrow F,
    ) -> JlrsResult<ArrayView<'borrow, T, Dim<IxDynImpl>>>
    where
        F: Frame<'fr>,
        T: ValidLayout,
    {
        let data = self.inline_data(frame)?;
        let shape = data.dimensions().as_slice().into_dimension().f();
        match ArrayView::from_shape(shape, data.into_slice()) {
            Ok(arr) => Ok(arr),
            Err(e) => other(e)?,
        }
    }

    fn array_view_mut<'fr: 'borrow, F>(
        self,
        frame: &'borrow mut F,
    ) -> JlrsResult<ArrayViewMut<'borrow, T, Dim<IxDynImpl>>>
    where
        F: Frame<'fr>,
        T: ValidLayout,
    {
        let data = self.inline_data_mut(frame)?;
        let shape = data.dimensions().as_slice().into_dimension().f();
        let raw = data.into_mut_slice();
        match ArrayViewMut::from_shape(shape, raw) {
            Ok(arr) => Ok(arr),
            Err(e) => other(e)?,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::NdArray;
    use jlrs::prelude::*;
    use ndarray::{ArrayView, ArrayViewMut, IxDyn};

    use std::cell::RefCell;

    thread_local! {
        pub static JULIA: RefCell<Julia> = RefCell::new(unsafe { Julia::init(32).unwrap() });
    }

    #[test]
    fn array_view() {
        JULIA.with(|j| {
            let mut julia = j.borrow_mut();

            julia
                .dynamic_frame(|_global, frame| {
                    let mut data = vec![1usize, 2, 3, 4, 5, 6];
                    let slice = &mut data.as_mut_slice();
                    let borrowed = Value::borrow_array(frame, slice, (3, 2))?;

                    let jl_array = borrowed.cast::<Array>()?;
                    let x = jl_array.inline_data::<usize, _>(frame)?[(1, 0)];

                    let array: ArrayView<usize, _> = jl_array.array_view(frame)?;
                    assert_eq!(array[IxDyn(&[1, 0])], x);

                    Ok(())
                })
                .unwrap();
        });
    }

    #[test]
    fn array_view_wrong_type() {
        JULIA.with(|j| {
            let mut julia = j.borrow_mut();

            julia
                .dynamic_frame(|_global, frame| {
                    let mut data = vec![1usize, 2, 3, 4, 5, 6];
                    let slice = &mut data.as_mut_slice();
                    let borrowed = Value::borrow_array(frame, slice, (3, 2))?;

                    let jl_array = borrowed.cast::<Array>()?;
                    let view: Result<ArrayView<isize, _>, _> = jl_array.array_view(frame);
                    assert!(view.is_err());
                    Ok(())
                })
                .unwrap();
        });
    }

    #[test]
    fn array_view_mut() {
        JULIA.with(|j| {
            let mut julia = j.borrow_mut();

            julia
                .dynamic_frame(|_global, frame| {
                    let mut data = vec![1usize, 2, 3, 4, 5, 6];
                    let slice = &mut data.as_mut_slice();
                    let borrowed = Value::borrow_array(frame, slice, (3, 2))?;

                    let jl_array = borrowed.cast::<Array>()?;
                    let mut inline = jl_array.inline_data_mut::<usize, _>(frame)?;
                    let x = inline[(1, 0)];

                    inline[(1, 0)] = x + 1;

                    let mut array: ArrayViewMut<usize, _> = jl_array.array_view_mut(frame)?;
                    assert_eq!(array[IxDyn(&[1, 0])], x + 1);
                    array[IxDyn(&[1, 0])] -= 1;

                    let inline = jl_array.inline_data_mut::<usize, _>(frame)?;
                    assert_eq!(inline[(1, 0)], x);
                    Ok(())
                })
                .unwrap();
        });
    }

    #[test]
    fn array_view_mut_wrong_type() {
        JULIA.with(|j| {
            let mut julia = j.borrow_mut();

            julia
                .dynamic_frame(|_global, frame| {
                    let mut data = vec![1usize, 2, 3, 4, 5, 6];
                    let slice = &mut data.as_mut_slice();
                    let borrowed = Value::borrow_array(frame, slice, (3, 2))?;

                    let jl_array = borrowed.cast::<Array>()?;
                    let view: Result<ArrayViewMut<isize, _>, _> = jl_array.array_view_mut(frame);
                    assert!(view.is_err());
                    Ok(())
                })
                .unwrap();
        });
    }

    #[test]
    fn typed_array_view() {
        JULIA.with(|j| {
            let mut julia = j.borrow_mut();

            julia
                .dynamic_frame(|_global, frame| {
                    let mut data = vec![1usize, 2, 3, 4, 5, 6];
                    let slice = &mut data.as_mut_slice();
                    let borrowed = Value::borrow_array(frame, slice, (3, 2))?;

                    let jl_array = borrowed.cast::<TypedArray<usize>>()?;
                    let x = jl_array.inline_data(frame)?[(1, 0)];

                    let array: ArrayView<usize, _> = jl_array.array_view(frame)?;
                    assert_eq!(array[IxDyn(&[1, 0])], x);

                    Ok(())
                })
                .unwrap();
        });
    }

    #[test]
    fn typed_array_view_mut() {
        JULIA.with(|j| {
            let mut julia = j.borrow_mut();

            julia
                .dynamic_frame(|_global, frame| {
                    let mut data = vec![1usize, 2, 3, 4, 5, 6];
                    let slice = &mut data.as_mut_slice();
                    let borrowed = Value::borrow_array(frame, slice, (3, 2))?;

                    let jl_array = borrowed.cast::<TypedArray<usize>>()?;
                    let mut inline = jl_array.inline_data_mut(frame)?;
                    let x = inline[(1, 0)];

                    inline[(1, 0)] = x + 1;

                    let mut array: ArrayViewMut<usize, _> = jl_array.array_view_mut(frame)?;
                    assert_eq!(array[IxDyn(&[1, 0])], x + 1);
                    array[IxDyn(&[1, 0])] -= 1;

                    let inline = jl_array.inline_data_mut(frame)?;
                    assert_eq!(inline[(1, 0)], x);
                    Ok(())
                })
                .unwrap();
        });
    }

    #[test]
    fn example() {
        JULIA.with(|j| {
            let mut julia = j.borrow_mut();

            julia
                .dynamic_frame(|_global, frame| {
                    let mut data = vec![1usize, 2, 3, 4, 5, 6];
                    let slice = &mut data.as_mut_slice();
                    let borrowed = Value::borrow_array(frame, slice, (3, 2))?;

                    let _array = borrowed.cast::<TypedArray<usize>>()?.array_view(frame)?;

                    Ok(())
                })
                .unwrap();
        });
    }
}
