#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Encode, Decode};

use frame_support::{
	decl_module, decl_storage, decl_event, decl_error, ensure, StorageValue, StorageDoubleMap,
	traits::Randomness, RuntimeDebug, dispatch::{DispatchError, DispatchResult},
};

use sp_io::hashing::blake2_128;

use frame_system::ensure_signed;


#[derive(Encode, Decode, Clone, RuntimeDebug, PartialEq, Eq)]
pub struct Kitty(pub [u8; 16]);

#[derive(Encode, Decode, Clone, Copy, RuntimeDebug, PartialEq, Eq)]
pub enum KittyGender {
	Male,
	Female,
}

impl Kitty {
	pub fn gender(&self) -> KittyGender {
		if self.0[0] % 2 == 0 {
			KittyGender::Male
		} else {
			KittyGender::Female
		}
	}
}

pub trait Trait: frame_system::Trait {
	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
	
	type Randomness: Randomness<Self::Hash>;
}

decl_storage! {
	trait Store for Module<T: Trait> as Kitties {
		/// Stores all the kitties, key is the kitty id
		pub Kitties get(fn kitties): double_map hasher(blake2_128_concat) T::AccountId, hasher(blake2_128_concat) u32 => Option<Kitty>;
		/// Stores the next kitty ID
		pub NextKittyId get(fn next_kitty_id): u32;
	}
}

decl_event! {
	pub enum Event<T> where
		<T as frame_system::Trait>::AccountId,
	{
		/// A kitty is created. \[owner, kitty_id-u32, kitty\]
		KittyCreated(AccountId, u32, Kitty),
		/// A new kitten is bred. \[owner, kitty_id-u32, kitty\]
		KittyBred(AccountId, u32, Kitty),
		/// A kitty is transferred. \[from, to, kitty_id-u32\]
		KittyTransferred(AccountId, AccountId, u32),
	}
}

decl_error! {
	pub enum Error for Module<T: Trait> {
		KittiesIdOverflow,
		InvalidKittyId,
		SameGender,
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;

		/// Create a new kitty
		#[weight = 1000] //weight is like the gas for this method, how long the method requires to run
		pub fn create(origin) {
			let sender = ensure_signed(origin)?;

			// TODO: refactor this method to use 
			// `Self::random_value` and `Self::get_next_kitty_id`
			let kitty_id = Self::get_next_kitty_id()?; //**** //current_id to kitty_id

			let dna = Self::random_value(&sender); //****	

			// Create and store kitty
			let kitty = Kitty(dna);
			//<Kitties<T>>::insert... exacly the same
			Kitties::<T>::insert(&sender, kitty_id, kitty.clone()); //current_id to kitty_id
			
			// Emit event
			Self::deposit_event(RawEvent::KittyCreated(sender, kitty_id, kitty)); //current_id to kitty_id
		}

		/// Breed kitties
		#[weight = 1000]
		pub fn breed(origin, kitty_id_1: u32, kitty_id_2: u32) {
			let sender = ensure_signed(origin)?;
			let kitty1 = Self::kitties(&sender, kitty_id_1).ok_or(Error::<T>::InvalidKittyId)?;
			let kitty2 = Self::kitties(&sender, kitty_id_2).ok_or(Error::<T>::InvalidKittyId)?;

			ensure!(kitty1.gender() != kitty2.gender(), Error::<T>::SameGender);
			 
			let kitty_id = Self::get_next_kitty_id()?;

			let kitty1_dna = kitty1.0;
			let kitty2_dna = kitty2.0;

			let selector = Self::random_value(&sender);
			let mut new_dna = [0u8; 16];

			// Combine parents and selector to create new kitty
			for i in 0..kitty1_dna.len() {
				new_dna[i] = combine_dna(kitty1_dna[i], kitty2_dna[i], selector[i]);
			}

			let new_kitty = Kitty(new_dna);

			Kitties::<T>::insert(&sender, kitty_id, &new_kitty);

			Self::deposit_event(RawEvent::KittyBred(sender, kitty_id, new_kitty));
		}
	


		// Transfer a kitty to new owner
		#[weight = 1000]
		pub fn transfer(origin, to: T::AccountId, kitty_id: u32) {
			let sender = ensure_signed(origin)?;

			Kitties::<T>::try_mutate_exists(sender.clone(), kitty_id, |kitty| -> DispatchResult {
				if sender == to {
					ensure!(kitty.is_some(), Error::<T>::InvalidKittyId);
					return Ok(());
				}

				let kitty = kitty.take().ok_or(Error::<T>::InvalidKittyId)?;

				Kitties::<T>::insert(&to, kitty_id, kitty);

				Self::deposit_event(RawEvent::KittyTransferred(sender, to, kitty_id));

				Ok(())
			})?;
		}
	}
}

pub fn combine_dna(dna1: u8, dna2: u8, selector: u8) -> u8 {
	// TODO: finish this implementation
	// selector[bit_index] == 0 -> use dna1[bit_index]
	// selector[bit_index] == 1 -> use dna2[bit_index]
	// e.g.
	// selector = 0b00000001
	// dna1		= 0b10101010
	// dna2		= 0b00001111
	// result	= 0b10101011
	(!selector & dna1) | (selector & dna2)
}



impl<T: Trait> Module<T> {
	// Generate a random 128bit value
	// Payload is basically random_seed from module and we have the sender as payload to make the number slightly more random
	fn get_next_kitty_id() -> sp_std::result::Result<u32, DispatchError> {
		NextKittyId::try_mutate(|next_id| -> sp_std::result::Result<u32, DispatchError> {
			let current_id = *next_id;
			// TODO: ensure kitty id does not overflow
			// return Err(Error::<T>::KittiesIdOverflow.into());
			*next_id = next_id.checked_add(1).ok_or(Error::<T>::KittiesIdOverflow)?;
			Ok(current_id)
		})
	}

		
	fn random_value(sender: &T::AccountId) -> [u8; 16] {
		// TODO: finish this implementation
		//Default::default()
		let payload = (
			T::Randomness::random_seed(),
			//<pallet_randomness_collective_flip::Module<T> as Randomness<T::Hash>>::random_seed(),
			&sender,
			<frame_system::Module<T>>::extrinsic_index(),
		);
		//let dna = payload.using_encoded(blake2_128);
		// this is basically encoding the payload result to a black2_128 (from sp_io) hashing function
		// this generates a 1 to 8 bit hash: the dna 
		payload.using_encoded(blake2_128)
	}	
}