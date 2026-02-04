use crate::{Error, http};
use lru::LruCache;
use pokerust::{Endpoint, FromId};
use rand::Rng;
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::sync::{Arc, LazyLock};
use tokio::sync::{Mutex, OnceCell, RwLock};

static TOTAL_POKEMON: LazyLock<OnceCell<i16>> = LazyLock::new(OnceCell::new);

static POKEMON_CACHE: LazyLock<RwLock<HashMap<i16, &'static pokerust::Pokemon>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

static POKEMON_SPECIES_CACHE: LazyLock<RwLock<HashMap<i16, &'static pokerust::PokemonSpecies>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

static IMAGE_BYTES_CACHE: LazyLock<Mutex<LruCache<i16, Arc<Vec<u8>>>>> =
    LazyLock::new(|| Mutex::new(LruCache::new(NonZeroUsize::new(50).unwrap())));

pub struct Pokemon {
    pub id: i16,
}

impl Pokemon {
    async fn get_pokemon(id: i16) -> Result<&'static pokerust::Pokemon, Error> {
        {
            let cache_read = POKEMON_CACHE.read().await;

            if let Some(pokemon) = cache_read.get(&id) {
                return Ok(*pokemon);
            }
        }

        // TODO: プログラム終了までキャッシュ。メモリリークのおそれ
        let pokemon = Box::leak(Box::new(pokerust::Pokemon::from_id(id)?));

        {
            let mut cache_write = POKEMON_CACHE.write().await;

            cache_write.insert(id, pokemon);
        }

        Ok(pokemon)
    }

    async fn get_species(id: i16) -> Result<&'static pokerust::PokemonSpecies, Error> {
        {
            let cache_read = POKEMON_SPECIES_CACHE.read().await;

            if let Some(species) = cache_read.get(&id) {
                return Ok(*species);
            }
        }

        // TODO: プログラム終了までキャッシュ。メモリリークのおそれ
        let species = Box::leak(Box::new(pokerust::PokemonSpecies::from_id(id)?));

        {
            let mut cache_write = POKEMON_SPECIES_CACHE.write().await;

            cache_write.insert(id, species);
        }

        Ok(species)
    }

    pub async fn name(&self) -> Result<Option<&'static str>, Error> {
        Ok(Self::get_species(self.id)
            .await?
            .names
            .iter()
            .find_map(|n| (n.language.name == "ja-hrkt").then_some(n.name.as_str())))
    }

    pub async fn flavor_text(&self) -> Result<Option<&'static str>, Error> {
        Ok(Self::get_species(self.id)
            .await?
            .flavor_text_entries
            .iter()
            .find_map(|f| (f.language.name == "ja-hrkt").then_some(f.flavor_text.as_str())))
    }

    pub async fn image_url(&self) -> Result<Option<&'static str>, Error> {
        let pokemon = Self::get_pokemon(self.id).await?;

        Ok(pokemon.sprites.front_default.as_deref())
    }

    pub async fn image_bytes(&self) -> Result<Option<Arc<Vec<u8>>>, Error> {
        let mut cache = IMAGE_BYTES_CACHE.lock().await;

        if let Some(bytes) = cache.get(&self.id) {
            return Ok(Some(Arc::clone(bytes)));
        }

        let Some(image_url) = self.image_url().await? else {
            return Ok(None);
        };

        let bytes = Arc::new(
            http::CLIENT
                .get(image_url)
                .send()
                .await?
                .bytes()
                .await?
                .to_vec(),
        );

        cache.put(self.id, bytes.clone());

        Ok(Some(bytes))
    }

    pub fn total() -> Result<i16, Error> {
        if let Some(&count) = TOTAL_POKEMON.get() {
            return Ok(count);
        }

        let list = pokerust::PokemonSpecies::list(0, 1)?;
        let count = list.count as i16;

        TOTAL_POKEMON.set(count)?;

        Ok(count)
    }

    pub fn random<R>(rng: &mut R) -> Result<Self, Error>
    where
        R: Rng,
    {
        let total = Self::total()?;
        let id = rng.gen_range(0..total);

        Ok(Self { id })
    }
}
